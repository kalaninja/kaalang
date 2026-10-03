use std::path::Path;

use kaalang_testing::probes::{branching, branching_with_work, data_branching, flow};

use super::*;
use crate::codegen::Bindings;

#[allow(clippy::too_many_lines)] // Compares diagnostics, semantics, Rust and topology against the complete reference.
fn compare(function: &syn::ItemFn) -> bool {
    let Ok(mut parsed) = crate::parse::flow(function) else {
        let Ok(analysis) = crate::analyze(function) else {
            return false;
        };
        if analysis.stages.is_empty() {
            return false;
        }
        let mut compared = compare_flow(function, &analysis.flow, Some(&analysis));
        for stage in &analysis.stages {
            let mut local = *stage.analysis.clone();
            local.stages = analysis.stages.clone();
            compared |= compare_flow(function, &local.flow, Some(&local));
        }
        return compared;
    };
    if crate::scope::resolve(&mut parsed).is_err() || crate::resolve::flow(&parsed).is_err() {
        return false;
    }
    compare_flow(function, &parsed, None)
}

#[allow(clippy::too_many_lines)] // Compares all observable phases with the complete reference.
fn compare_flow(function: &syn::ItemFn, parsed: &Flow, existing: Option<&crate::Analysis>) -> bool {
    let ordinary = crate::analyze::flow(parsed, true);
    let symbolic = analyze(parsed, true);
    let (mut symbolic, groups, merges) = match (ordinary, symbolic) {
        (Err(expected), Err(actual)) => {
            assert_eq!(
                actual.to_string(),
                expected.to_string(),
                "{}",
                function.sig.ident
            );
            return true;
        }
        (Err(error), Ok(_)) => panic!(
            "{}: symbolic analysis accepted: {error}",
            function.sig.ident
        ),
        (Ok(_), Err(error)) => panic!(
            "{}: symbolic analysis rejected: {error}",
            function.sig.ident
        ),
        (
            Ok((executions, expected_groups, expected_merges, passes)),
            Ok((symbolic, groups, merges)),
        ) => {
            assert_eq!(groups, expected_groups, "{}: groups", function.sig.ident);
            assert_eq!(merges, expected_merges, "{}: merges", function.sig.ident);
            assert_eq!(
                symbolic.len(),
                executions.len(),
                "{}: count",
                function.sig.ident
            );
            assert_eq!(
                symbolic.enumerate(),
                executions,
                "{}: executions",
                function.sig.ident
            );
            let mut availability = symbolic.clone();
            for name in parsed
                .flow_inputs
                .iter()
                .chain(parsed.blocks.iter().flat_map(|block| &block.outputs))
            {
                let expected = executions
                    .iter()
                    .filter(|execution| {
                        matches!(execution.outcome, ExecutionOutcome::Return { .. })
                    })
                    .all(|execution| crate::stage::available(parsed, execution, name));
                assert_eq!(
                    availability.available_on_completion(parsed, name),
                    expected,
                    "{}: common wire {name}",
                    function.sig.ident
                );
            }
            let reference_plan = crate::plan::flow(parsed, &executions, &merges, passes);
            let mut compact = existing
                .cloned()
                .unwrap_or_else(|| crate::analyze(function).expect("the flow analyzes"));
            let mut symbolic = symbolic;
            compact.execution_plan = symbolic.plan(parsed, &merges);
            compact.executions = crate::Executions::factored(symbolic.clone());
            let mut reference = compact.clone();
            reference.execution_plan = reference_plan;
            reference.executions = crate::Executions::enumerated(executions);
            let tokens = |analysis: &crate::Analysis| {
                crate::codegen::flow(
                    &analysis.flow,
                    &analysis.execution_plan,
                    &Bindings::new(analysis),
                )
                .to_string()
            };
            assert_eq!(
                tokens(&compact),
                tokens(&reference),
                "{}: Rust",
                function.sig.ident
            );
            for collapsed in [false, true] {
                let actual = crate::project(&compact, collapsed);
                let expected = crate::project(&reference, collapsed);
                assert_eq!(
                    actual.connections, expected.connections,
                    "{}: connections",
                    function.sig.ident
                );
                assert_eq!(
                    actual.order, expected.order,
                    "{}: order",
                    function.sig.ident
                );
                assert_eq!(
                    actual.vertices, expected.vertices,
                    "{}: vertices",
                    function.sig.ident
                );
                assert_eq!(
                    actual
                        .junctions
                        .iter()
                        .map(|junction| &junction.merges)
                        .collect::<Vec<_>>(),
                    expected
                        .junctions
                        .iter()
                        .map(|junction| &junction.merges)
                        .collect::<Vec<_>>(),
                    "{}: junctions",
                    function.sig.ident
                );
                assert_eq!(
                    actual.back_edges, expected.back_edges,
                    "{}: back edges",
                    function.sig.ident
                );
                let cycles = |topology: &crate::topology::Topology| {
                    topology
                        .cycles
                        .iter()
                        .map(|cycle| (cycle.header, cycle.tail, cycle.prefer_left))
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    cycles(&actual),
                    cycles(&expected),
                    "{}: cycle tails",
                    function.sig.ident
                );
            }
            (symbolic, groups, merges)
        }
    };
    let plan = symbolic.plan(parsed, &merges);
    assert!(symbolic.verify(parsed, &plan, &merges));
    assert_eq!(
        groups,
        crate::analyze::flow(parsed, true)
            .expect("the ordinary analysis succeeds")
            .1
    );
    true
}

#[test]
fn symbolic_analysis_matches_the_complete_fixture_corpus() {
    use kaalang_testing::corpus::{self, Suite};

    fn rejected(directory: &Path, compared: &mut usize) {
        for entry in std::fs::read_dir(directory).expect("the test tree exists") {
            let path = entry.expect("the directory entry exists").path();
            if path.is_dir() {
                rejected(&path, compared);
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && path
                    .components()
                    .any(|part| part.as_os_str() == "compile_fail")
            {
                let source = std::fs::read_to_string(&path).expect("the fixture is readable");
                if let Ok(file) = syn::parse_file(&source) {
                    for function in crate::flows(&file.items) {
                        *compared += usize::from(compare(&function));
                    }
                }
            }
        }
    }
    let mut compared = 0;
    for (_, function, _) in corpus::corpus(Suite::All) {
        compared += usize::from(compare(&function));
    }
    rejected(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../kaalang/tests"),
        &mut compared,
    );
    assert!(
        compared > 100,
        "{compared} comparisons lost fixture coverage"
    );
}

#[test]
fn symbolic_analysis_matches_all_generated_combinations() {
    for levels in 4..=8 {
        for source in [
            branching(levels),
            data_branching(levels),
            branching_with_work(levels),
        ] {
            assert!(compare(&flow(&source)));
        }
    }
    assert!(compare(&flow(&data_branching(12))));
}

#[test]
fn arbitrary_branch_work_and_captures_preserve_rejection_priority() {
    let function = flow(&branching_with_work(4));
    for position in 0..function.block.stmts.len() - 1 {
        let mut reordered = function.clone();
        reordered.block.stmts.swap(position, position + 1);
        compare(&reordered);
        let mut removed = function.clone();
        removed.block.stmts.remove(position);
        compare(&removed);
    }
    let nested = branching_with_work(4).replace(
        "let data_0_0 = |case_0_0, &prepared, &config| *prepared + *config;",
        "let (nested_yes, nested_no) = |case_0_0, &config| *config > 1;\n    #[action(\"Nested yes.\")]\n    let data_0_0 = |nested_yes, &prepared, &config| *prepared + *config;\n    #[action(\"Nested no.\")]\n    let data_0_0 = |nested_no, &prepared, &config| *prepared + *config + 1;"
    ).replace("#[action(\"Prepare branch 0/0.\")]", "#[question(\"Nested work?\")]");
    assert!(compare(&flow(&nested)));
}

#[test]
fn transitive_dependencies_can_require_more_than_two_changed_answers() {
    let mut body = String::from("#[kaalang] fn conditional(flag: bool, seed: usize) -> usize {\n");
    let mut previous = "seed".to_owned();
    for level in 0..6 {
        use std::fmt::Write;
        writeln!(body, "#[question(\"Keep the earlier value?\")] let (reset_{level}, keep_{level}) = |&flag| *flag;\n#[action(\"Reset.\")] let step_{level} = |reset_{level}, &seed| *seed;\n#[action(\"Keep.\")] let step_{level} = |keep_{level}, &{previous}| *{previous} + 1;").expect("the source is writable");
        previous = format!("step_{level}");
    }
    body.push_str("|step_5| return step_5; }");
    assert!(compare(&flow(&body)));
}

#[test]
fn the_symbolic_verifier_rejects_a_join_dropping_its_binding() {
    let function = flow(&data_branching(4));
    let mut parsed = crate::parse::flow(&function).expect("the probe parses");
    crate::scope::resolve(&mut parsed).expect("the probe resolves");
    crate::resolve::flow(&parsed).expect("the probe validates its names");
    let (mut symbolic, _, merges) = analyze(&parsed, true).expect("the probe analyzes");
    let mut plan = symbolic.plan(&parsed, &merges);
    let crate::ExecutionPlan::End { body, .. } = &mut plan else {
        unreachable!()
    };
    let crate::ExecutionPlan::Question { joins, .. } = body.as_mut() else {
        unreachable!()
    };
    joins[0].wires.clear();
    assert!(!symbolic.verify(&parsed, &plan, &merges));
}

#[test]
fn cycles_and_stage_boundaries_match_complete_histories() {
    for levels in 1..=5 {
        for source in [
            kaalang_testing::probes::cyclic_branching_with_work(levels),
            kaalang_testing::probes::staged_branching_with_work(levels),
        ] {
            let function = flow(&source);
            crate::analyze(&function)
                .unwrap_or_else(|error| panic!("{}: {error}\n{source}", function.sig.ident));
            assert!(compare(&function));
        }
    }
    for (_, source, _) in kaalang_testing::probes::accepted() {
        assert!(compare(&flow(&source)));
    }
}

#[test]
fn nested_cycle_frames_and_rejected_body_mutations_match_the_reference() {
    fn body(function: &mut syn::ItemFn) -> &mut syn::Block {
        let syn::Stmt::Local(local) = &mut function.block.stmts[0] else {
            panic!("the cycle is a binding")
        };
        let syn::Expr::Closure(closure) = local
            .init
            .as_mut()
            .expect("the cycle has a closure")
            .expr
            .as_mut()
        else {
            panic!("the cycle is a closure")
        };
        let syn::Expr::Block(block) = closure.body.as_mut() else {
            panic!("the cycle has a body")
        };
        &mut block.block
    }
    let source = kaalang_testing::probes::cyclic_branching_with_work(3);
    let nested = source
        .replace(
            "let (left, right) = || {",
            "let (left, right) = || { #[cycle(\"Inner work.\")] let (left, right) = || {",
        )
        .replace(
            "\n};\n#[action(\"Use the left result.\")]",
            "\n};\n};\n#[action(\"Use the left result.\")]",
        );
    assert!(compare(&flow(&nested)));
    let mut function = flow(&source);
    let original = body(&mut function).clone();
    for position in 0..original.stmts.len() - 1 {
        for remove in [false, true] {
            let mut changed = original.clone();
            if remove {
                changed.stmts.remove(position);
            } else {
                changed.stmts.swap(position, position + 1);
            }
            let mut mutated = function.clone();
            *body(&mut mutated) = changed;
            compare(&mutated);
        }
    }
}

#[test]
fn the_symbolic_verifier_rejects_exporting_to_an_inactive_cycle() {
    fn change(plan: &mut crate::ExecutionPlan) {
        use crate::ExecutionPlan;
        match plan {
            ExecutionPlan::Export { target, .. } => *target = usize::MAX,
            ExecutionPlan::End { body, .. } | ExecutionPlan::Cycle { body, .. } => change(body),
            ExecutionPlan::Action { next, .. } | ExecutionPlan::Call { next, .. } => change(next),
            ExecutionPlan::Question {
                branches, joins, ..
            }
            | ExecutionPlan::Choice {
                branches, joins, ..
            } => {
                for branch in branches {
                    change(branch);
                }
                for join in joins {
                    change(&mut join.next);
                }
            }
            ExecutionPlan::Return { .. }
            | ExecutionPlan::Continue { .. }
            | ExecutionPlan::Yield { .. } => {}
        }
    }
    let function = flow(&kaalang_testing::probes::cyclic_branching_with_work(5));
    let analysis = crate::analyze(&function).expect("the probe analyzes");
    let mut symbolic = analysis
        .executions
        .symbolic()
        .expect("the probe is factored")
        .clone();
    let mut plan = analysis.execution_plan.clone();
    change(&mut plan);
    assert!(!symbolic.verify(&analysis.flow, &plan, &analysis.merges));
}
