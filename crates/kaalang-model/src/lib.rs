//! Parses and validates kaalang flows into the semantic model.

use syn::{ItemFn, Result};

mod analyze;
mod choice;
mod construct;
pub mod geometry;
mod model;
mod parse;
#[cfg(test)]
mod performance;
mod plan;
mod resolve;
mod scope;
pub mod topology;

pub use choice::{choice_match, is_todo_body};
pub use construct::{Arrangement, Contour, Route, Run, RunLine, Side};
pub use model::{
    Block, BlockKind, Branch, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup,
    Execution, ExecutionOutcome, ExecutionPlan, Flow, Input, Join, JoinTarget, ProducerId,
    QuestionBranch, SemanticModel, WireMerge,
};

/// Diagram projection options. Analysis and expanded-topology validation are
/// identical for both views.
#[derive(Clone, Copy, Default)]
pub struct BuildOptions {
    pub collapse_loops: bool,
}

/// Builds the validated semantic model for one kaalang flow function.
///
/// # Errors
///
/// Returns the first violation found while parsing block syntax, resolving
/// wires to their producers, walking every possible execution, or deriving
/// convergence groups, spanned at the offending token so callers can report it
/// against the authored source.
///
/// Returns an impossible-topology error, at the block it concerns, when the
/// flow's required connections have no conforming diagram under RFC 0002, and
/// an internal construction error when the independent check rejects the
/// arrangement the search returned.
pub fn build(function: &ItemFn) -> Result<SemanticModel> {
    build_with_options(function, BuildOptions::default())
}

/// Builds a validated semantic model in the requested loop presentation.
/// Expanded topology is always constructed first, so collapsing cannot hide an
/// invalid cycle body or an unrealizable expanded diagram.
///
/// # Errors
///
/// Returns the same parsing, validation, and topology errors as [`build`].
pub fn build_with_options(function: &ItemFn, options: BuildOptions) -> Result<SemanticModel> {
    let mut flow = parse::flow(function)?;
    scope::resolve(&mut flow)?;
    resolve::flow(&flow)?;
    let (executions, convergence_groups, merges) = analyze::flow(&flow)?;
    let execution_plan = plan::flow(&flow, &executions, &merges);
    let analyzed = topology::Analyzed {
        flow: &flow,
        executions: &executions,
        merges: &merges,
        execution_plan: &execution_plan,
    };
    let expanded = topology::project(&analyzed, false);
    let expanded_arrangement = construct::construct(&flow, &merges, &expanded)?;
    let (topology, arrangement) = if options.collapse_loops {
        let topology = topology::project(&analyzed, true);
        let arrangement = construct::construct(&flow, &merges, &topology)?;
        (topology, arrangement)
    } else {
        (expanded, expanded_arrangement)
    };

    Ok(SemanticModel {
        name: function.sig.ident.clone(),
        parameters: function.sig.inputs.iter().cloned().collect(),
        return_type: function.sig.output.clone(),
        flow,
        execution_plan,
        executions,
        convergence_groups,
        merges,
        topology,
        arrangement,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use quote::ToTokens;
    use syn::{FnArg, ItemFn, Pat, ReturnType, Type, parse_quote};

    use super::{
        BlockKind, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup,
        ExecutionOutcome, ExecutionPlan, ProducerId, SemanticModel, build,
    };

    /// The flow declared by `crates/kaalang/tests/<dir>/<stem>.rs`, named after the file.
    macro_rules! fixture {
        ($dir:literal, $stem:literal) => {
            fixture(
                include_str!(concat!("../../kaalang/tests/", $dir, "/", $stem, ".rs")),
                $stem,
            )
        };
    }

    /// How many times the plan emits one block. The end block is emitted once
    /// when the plan is rooted at it.
    fn count_block(model: &SemanticModel, target: usize) -> usize {
        crate::plan::verify::emitted(&model.execution_plan)
            .iter()
            .filter(|&&block| block == target)
            .count()
    }

    /// The flow named `flow` in a fixture file's source.
    pub(crate) fn fixture(source: &str, flow: &str) -> ItemFn {
        let file = syn::parse_file(source).expect("the fixture parses");
        file.items
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == flow => Some(function),
                _ => None,
            })
            .expect("the fixture declares its flow")
    }

    fn end_body(plan: &ExecutionPlan) -> &ExecutionPlan {
        let ExecutionPlan::End { body, .. } = plan else {
            panic!("the verified plan must be rooted at end")
        };
        body
    }

    /// The message a rejected flow is built with.
    pub(crate) fn message(function: &ItemFn) -> String {
        build(function)
            .err()
            .expect("the flow is rejected")
            .to_string()
    }

    /// The diagnostic every flow that opens a second branch too early reports.
    pub(crate) fn branch_placement(kind: &str, description: &str) -> String {
        format!(
            "this kaalang block runs while the branches of the {kind} `{description}` are still separate; give it an input from one branch, or merge those branches above it"
        )
    }

    fn group(
        branching_block: usize,
        branches: &[usize],
        continuation: &[usize],
        entries: &[usize],
    ) -> ConvergenceGroup {
        ConvergenceGroup {
            branching_block,
            branches: branches.to_vec(),
            continuation: continuation.to_vec(),
            entries: entries.to_vec(),
        }
    }

    /// Builds the flow and checks its exact convergence groups. The end block
    /// is never a continuation member, so no group may mention the last block.
    fn assert_groups(function: &ItemFn, expected: &[ConvergenceGroup]) {
        let model = build(function).expect("the flow is valid");
        let end = model.flow.blocks.len() - 1;
        assert_eq!(model.convergence_groups, expected);
        for recorded in &model.convergence_groups {
            assert!(
                !recorded.continuation.contains(&end),
                "the end block is not a computational continuation member"
            );
        }
    }

    #[test]
    fn cycle_examples_have_one_verified_body_per_authored_block() {
        for (name, source) in [
            (
                "count_to",
                include_str!("../../kaalang/tests/loop/behavior/count_to.rs"),
            ),
            (
                "binary_search",
                include_str!("../../kaalang/tests/gallery/binary_search/mod.rs"),
            ),
            (
                "nested_search",
                include_str!("../../kaalang/tests/loop/behavior/nested_search.rs"),
            ),
            (
                "empty_loop",
                include_str!("../../kaalang/tests/loop/behavior/empty_loop.rs"),
            ),
            (
                "repeat_until_break",
                include_str!("../../kaalang/tests/loop/behavior/repeat_until_break.rs"),
            ),
            (
                "nested_loops",
                include_str!("../../kaalang/tests/loop/behavior/nested_loops.rs"),
            ),
            (
                "nested_unconditional_loops",
                include_str!("../../kaalang/tests/loop/behavior/nested_unconditional_loops.rs"),
            ),
        ] {
            let model =
                build(&fixture(source, name)).unwrap_or_else(|error| panic!("{name}: {error}"));
            for block in 0..model.flow.blocks.len() {
                assert_eq!(count_block(&model, block), 1, "{name}: block {block}");
            }
        }
    }

    #[test]
    fn a_flow_input_named_end_still_needs_an_explicit_return() {
        let function: ItemFn = parse_quote! {
            fn identity(end: u8) -> u8 {
                |end| return end;
            }
        };

        let model = build(&function).expect("the zero-computation flow is valid");
        assert_eq!(model.flow.flow_inputs, ["end"]);
        assert_eq!(model.flow.blocks.len(), 2);
        assert_eq!(model.executions.len(), 1);
        assert_eq!(model.executions[0].blocks, [0]);
        assert_eq!(model.executions[0].dependencies.len(), 1);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Return { index: 0 }
        ));
    }

    #[test]
    fn a_flow_without_a_root_return_is_rejected() {
        assert_eq!(
            message(&parse_quote! {
                fn nothing() {}
            }),
            "this kaalang execution reaches the end of the flow without `return`"
        );
    }

    #[test]
    fn an_unconditional_cycle_may_repeat_without_reaching_end() {
        let function: ItemFn = parse_quote! {
            fn forever() -> usize {
                #[cycle("Repeat forever")]
                || {};
            }
        };

        let model = build(&function).expect("the diverging flow is valid");
        assert_eq!(model.flow.blocks[0].kind, BlockKind::Loop);
        assert_eq!(
            model.executions[0].outcome,
            ExecutionOutcome::Repeat { loop_index: 0 }
        );
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Loop { index: 0, body, .. }
                if matches!(body.as_ref(), ExecutionPlan::Repeat { index: 0 })
        ));

        let model = build(&fixture(
            include_str!("../../kaalang/tests/loop/behavior/repeat_until_break.rs"),
            "repeat_until_break",
        ))
        .expect("the cycle may either repeat or finish");
        assert_eq!(
            model
                .executions
                .iter()
                .map(|execution| execution.outcome)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                ExecutionOutcome::Return { block_index: 5 },
                ExecutionOutcome::Repeat { loop_index: 1 },
            ])
        );
    }

    #[test]
    fn wildcard_is_not_a_flow_input_but_underscore_name_is() {
        let function: ItemFn = parse_quote! {
            fn discard(_: u8, _value: u8) {
                return;
            }
        };

        let model = build(&function).expect("both ignored parameter forms are valid");
        assert_eq!(model.flow.flow_inputs.len(), 1);
        assert_eq!(model.flow.flow_inputs[0], "_value");
        assert!(matches!(
            model.parameters.as_slice(),
            [FnArg::Typed(wildcard), FnArg::Typed(named)]
                if matches!(wildcard.pat.as_ref(), Pat::Wild(_))
                    && matches!(named.pat.as_ref(), Pat::Ident(binding) if binding.ident == "_value")
        ));
    }

    #[test]
    fn preserves_authored_descriptions_and_case_order() {
        let function: ItemFn = parse_quote! {
            fn choose(input: usize) -> usize {
                #[choice("  Choose a path  ")]
                #[case("Первый")]
                #[case("Second & final")]
                let (left, right) = |input| {
                    match input {
                        0 => input,
                        _ => input,
                    }
                };

                #[action("Use the first path")]
                let result = |left| { left };

                #[action("Use the second path")]
                let result = |right| { right };

                |result| return result;
            }
        };

        let model = build(&function).expect("the flow is valid");
        let choice = &model.flow.blocks[0];

        assert_eq!(model.name, "choose");
        assert_eq!(model.parameters.len(), 1);
        let FnArg::Typed(parameter) = &model.parameters[0] else {
            panic!("the parameter must be typed")
        };
        let Pat::Ident(parameter) = parameter.pat.as_ref() else {
            panic!("the parameter must retain its authored name")
        };
        assert_eq!(parameter.ident, "input");
        let ReturnType::Type(_, return_type) = &model.return_type else {
            panic!("the return type must be preserved")
        };
        let Type::Path(return_type) = return_type.as_ref() else {
            panic!("the return type must remain a path")
        };
        assert!(return_type.path.is_ident("usize"));
        assert_eq!(choice.kind, BlockKind::Choice);
        assert_eq!(choice.description.as_deref(), Some("  Choose a path  "));
        assert_eq!(choice.case_descriptions, ["Первый", "Second & final"]);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Choice {
                branches,
                joins,
                ..
            } if branches.len() == 2 && joins.len() == 1
        ));
    }

    #[test]
    fn records_alternative_producers_against_each_execution() {
        let function: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                let (yes, no) = |condition| { condition };

                #[action("Build the yes value")]
                let selected = |yes| { 1 };

                #[action("Build the no value")]
                let selected = |no| { 2 };

                |selected| return selected;
            }
        };

        let model = build(&function).expect("the alternative producers are valid");
        assert_eq!(model.flow.blocks[1].outputs[0], "selected");
        assert_eq!(model.flow.blocks[2].outputs[0], "selected");
        assert_eq!(model.flow.blocks[3].inputs[0].ident, "selected");

        let [yes, no] = model.executions.as_slice() else {
            panic!("a question has two executions")
        };
        assert_eq!(
            yes.branches,
            [BranchSelection {
                block: 0,
                branch: 0
            }]
        );
        assert_eq!(
            no.branches,
            [BranchSelection {
                block: 0,
                branch: 1
            }]
        );
        assert_eq!(yes.blocks, [0, 1, 3]);
        assert_eq!(no.blocks, [0, 2, 3]);
        let shared = CaptureId { block: 3, input: 0 };
        assert!(yes.dependencies.contains(&CaptureDependency {
            producer: ProducerId::BlockOutput {
                block: 1,
                output: 0
            },
            capture: shared,
        }));
        assert!(no.dependencies.contains(&CaptureDependency {
            producer: ProducerId::BlockOutput {
                block: 2,
                output: 0
            },
            capture: shared,
        }));
    }

    #[test]
    fn records_one_shared_consumer_and_orders_join_wires_by_producer() {
        let function: ItemFn = parse_quote! {
            fn choose(condition: bool) -> (u32, u32) {
                #[question("Choose values")]
                let (yes, no) = |condition| { condition };

                #[action("Build the yes values")]
                let (first, second) = |yes| { (1, 2) };

                #[action("Build the no values")]
                let (first, second) = |no| { (3, 4) };

                |second, first| return (first, second);
            }
        };

        let model = build(&function).expect("the alternative producers are valid");
        let ExecutionPlan::Question { joins, .. } = end_body(&model.execution_plan) else {
            panic!("the question must record its join")
        };
        let [join] = joins.as_slice() else {
            panic!("the question must record one join")
        };

        assert_eq!(join.branches, [0, 1]);
        assert_eq!(
            join.wires
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
        assert_eq!(count_block(&model, 3), 1);
        assert_eq!(count_block(&model, 4), 1);
    }

    #[test]
    fn blocks_sharing_no_wire_form_one_execution_in_source_order() {
        let function: ItemFn = parse_quote! {
            fn pair(input: u32) -> (u32, u32) {
                #[action("Produce the first result")]
                let first = |&input| { *input };

                #[action("Produce the second result")]
                let second = |&input| { *input + 1 };

                |first, second| return (first, second);
            }
        };

        let model = build(&function).expect("independent borrowers are valid");
        assert_eq!(model.executions.len(), 1);
        assert_eq!(model.executions[0].blocks, [0, 1, 2]);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Action { index: 0, next }
                if matches!(next.as_ref(), ExecutionPlan::Action { index: 1, next }
                    if matches!(next.as_ref(), ExecutionPlan::Return { index: 2 }))
        ));
    }

    /// The second question opens its branches only once the first question's
    /// branches have merged, so both bodies are still emitted once.
    #[test]
    fn successive_questions_share_every_body() {
        let function: ItemFn = parse_quote! {
            fn route(left: bool, right: bool) -> u8 {
                #[question("Choose the right value")]
                let (x, y) = |right| { right };
                #[action("Build the first right value")]
                let value = |x| { 10 };
                #[action("Build the second right value")]
                let value = |y| { 20 };
                #[question("Choose the left path")]
                let (a, b) = |left| { left };
                #[action("Use the left path")]
                let result = |a, value| { value + 1 };
                #[action("Use the other left path")]
                let result = |b, value| { value + 2 };
                |result| return result;
            }
        };

        let model = build(&function).expect("the merged value precedes the second question");
        assert_eq!(model.executions.len(), 4);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Question { index: 0, .. }
        ));
        for block in 0..model.flow.blocks.len() {
            assert_eq!(count_block(&model, block), 1);
        }
    }

    #[test]
    fn shared_computation_runs_where_it_is_written() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, seed: u32) -> u32 {
                #[action("Prepare a shared value")]
                let prepared = |seed| { seed };

                #[question("Which way?")]
                let (yes, no) = |condition| { condition };

                #[action("Use it on the yes branch")]
                let result = |yes, &prepared| { *prepared };

                #[action("Use it on the no branch")]
                let result = |no, prepared| { prepared + 1 };

                |result| return result;
            }
        };

        let model = build(&function).expect("the flow is valid");
        assert_eq!(model.executions.len(), 2);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Action { index: 0, next }
                if matches!(next.as_ref(), ExecutionPlan::Question { index: 1, joins, .. } if !joins.is_empty())
        ));
    }

    #[test]
    fn records_nested_join_topology() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, value: usize) -> usize {
                #[question("Take the branching path?")]
                let (yes, no) = |condition, &value| { condition };

                #[choice("Which branch?")]
                #[case("First")]
                #[case("Second")]
                #[case("Third")]
                let (first, second, third) = |yes, value| {
                    match value {
                        0 => (),
                        1 => (),
                        _ => (),
                    }
                };

                #[action("Build the first value")]
                let selected = |first| { 1 };

                #[action("Build the second value")]
                let selected = |second| { 2 };

                #[action("Produce the third result")]
                let result = |third| { 3 };

                #[action("Produce the selected result")]
                let result = |selected| { selected };

                #[action("Produce the no-branch result")]
                let result = |no| { 0 };

                |result| return result;
            }
        };

        let model = build(&function).expect("the flow is valid");
        assert_eq!(model.executions.len(), 4);
        let ExecutionPlan::Question {
            index,
            branches: [yes, no],
            joins,
        } = end_body(&model.execution_plan)
        else {
            panic!("the root question must join its results")
        };
        let [root_join] = joins.as_slice() else {
            panic!("the root question must record one join")
        };
        assert_eq!(*index, 0);
        assert!(matches!(
            no.plan.as_ref(),
            ExecutionPlan::Action { index: 6, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["result"])
        ));
        assert_eq!(root_join.branches, [0, 1]);
        assert_eq!(root_join.wires, ["result"]);
        assert!(matches!(
            root_join.next.as_ref(),
            ExecutionPlan::Return { index: 7 }
        ));

        let ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } = yes.plan.as_ref()
        else {
            panic!("the yes branch must contain the choice")
        };
        assert_eq!(*index, 1);
        assert_eq!(branches.len(), 3);
        assert!(matches!(
            branches[0].plan.as_ref(),
            ExecutionPlan::Action { index: 2, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["selected"])
        ));
        assert!(matches!(
            branches[1].plan.as_ref(),
            ExecutionPlan::Action { index: 3, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["selected"])
        ));
        assert!(matches!(
            branches[2].plan.as_ref(),
            ExecutionPlan::Action { index: 4, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["result"])
        ));
        let [selected_join] = joins.as_slice() else {
            panic!("the first two cases share one nested join")
        };
        assert_eq!(selected_join.branches, [0, 1]);
        assert_eq!(selected_join.wires, ["selected"]);
        assert!(matches!(
            selected_join.next.as_ref(),
            ExecutionPlan::Action { index: 5, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["result"])
        ));
        assert_eq!(count_block(&model, 8), 1);
    }

    /// A branch output is an ordinary wire, but it stays branch-local: reading
    /// it again below its own merge is rejected like any other branch-local
    /// value, and not by a rule of its own.
    #[test]
    fn rejects_a_branch_output_read_after_convergence() {
        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/branch_output_after_convergence.rs"
        );
        assert_eq!(
            message(&fixture(source, "invalid")),
            "this kaalang block must finish before the `selected` wire merge, but it waits for a value from after that merge"
        );
    }

    /// The second question opens its own branches while the first question's
    /// are still separate, which is reported before the three executions that
    /// would reach the flow boundary without returning.
    #[test]
    fn rejects_a_conjunction_of_independent_branch_outputs() {
        let function: ItemFn = parse_quote! {
            fn both(left: bool, right: bool) -> u8 {
                #[question("Left?")]
                let (a, _b) = |left| { left };
                #[question("Right?")]
                let (c, _d) = |right| { right };
                #[action("Both")]
                let result = |a, c| { 1 };
                |result| return result;
            }
        };

        assert_eq!(message(&function), branch_placement("question", "Left?"));
    }

    #[test]
    fn rejects_branch_local_access_to_a_name_with_alternative_producers() {
        let function: ItemFn = parse_quote! {
            fn route(outer: bool, inner: bool) -> u8 {
                #[question("Choose the source")]
                let (yes, no) = |outer| { outer };

                #[action("Prepare the nested branch")]
                let (trigger, branch_gate) = |no| { ((), ()) };

                #[question("Choose the control output")]
                let (shared, skip) = |trigger, inner| { inner };

                #[action("Produce borrowable data")]
                let (shared, borrow_gate) = |yes| { ((), ()) };

                #[action("Consume the nested control")]
                let result = |shared, branch_gate| { 1 };

                #[action("Consume the other nested control")]
                let result = |skip, branch_gate| { 2 };

                #[action("Borrow only the action output")]
                let result = |&shared, borrow_gate| { 3 };

                |result| return result;
            }
        };

        assert_eq!(
            message(&function),
            "a kaalang wire with alternative producers merges before every capture; a branch-local value needs its own name"
        );
    }

    #[test]
    fn one_choice_may_own_disjoint_joins_inside_a_full_join() {
        let function: ItemFn = parse_quote! {
            fn route(value: u8) -> u8 {
                #[choice("Which group?")]
                #[case("First of the left group")]
                #[case("Second of the left group")]
                #[case("Direct result")]
                #[case("First of the right group")]
                #[case("Second of the right group")]
                let (a, b, done, c, d) = |value| {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        _ => (),
                    }
                };

                #[action("Build the left value from a")]
                let left = |a| { 1 };

                #[action("Build the left value from b")]
                let left = |b| { 2 };

                #[action("Produce the direct result")]
                let result = |done| { 3 };

                #[action("Build the right value from c")]
                let right = |c| { 4 };

                #[action("Build the right value from d")]
                let right = |d| { 5 };

                #[action("Use the left value")]
                let result = |left| { left };

                #[action("Use the right value")]
                let result = |right| { right };

                |result| return result;
            }
        };

        let model = build(&function).expect("two disjoint joins are valid");
        assert_eq!(model.executions.len(), 5);
        let ExecutionPlan::Choice {
            branches, joins, ..
        } = end_body(&model.execution_plan)
        else {
            panic!("the root must be the choice")
        };
        assert!(matches!(
            branches[2].plan.as_ref(),
            ExecutionPlan::Action { index: 3, next }
                if matches!(next.as_ref(), ExecutionPlan::Yield { wires, .. } if wires == &["result"])
        ));
        assert_eq!(joins.len(), 3);
        let left = joins.iter().find(|join| join.wires == ["left"]).unwrap();
        let right = joins.iter().find(|join| join.wires == ["right"]).unwrap();
        let result = joins.iter().find(|join| join.wires == ["result"]).unwrap();
        assert_eq!(left.branches, [0, 1]);
        assert_eq!(right.branches, [3, 4]);
        assert_eq!(result.branches, [0, 1, 2, 3, 4]);
        assert!(matches!(
            left.next.as_ref(),
            ExecutionPlan::Action { index: 6, .. }
        ));
        assert!(matches!(
            right.next.as_ref(),
            ExecutionPlan::Action { index: 7, .. }
        ));
        assert!(matches!(
            result.next.as_ref(),
            ExecutionPlan::Return { index: 8 }
        ));
        for block in 0..10 {
            assert_eq!(count_block(&model, block), 1, "block {block}");
        }
        assert!(
            model
                .executions
                .iter()
                .all(|execution| execution.blocks.contains(&0))
        );
    }

    #[test]
    fn nested_branches_cannot_separate_a_merge() {
        for source in [
            include_str!(
                "../../kaalang/tests/wire/compile_fail/nested_branch_passes_a_case_join.rs"
            ),
            include_str!(
                "../../kaalang/tests/wire/compile_fail/nested_choice_passes_a_question_join.rs"
            ),
            include_str!(
                "../../kaalang/tests/wire/compile_fail/terminal_branch_separates_a_merge.rs"
            ),
        ] {
            assert_eq!(
                message(&fixture(source, "invalid")),
                "branches reaching the `shared` wire merge must be adjacent, including nested branches"
            );
        }
    }

    #[test]
    fn a_partial_continuation_may_feed_a_wider_merge() {
        build(&fixture!(
            "wire/behavior",
            "nested_branch_passes_a_question_join"
        ))
        .expect("all result producers meet at one valid merge");

        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/nested_branch_passes_a_case_join.rs"
        )
        .replace("let (join, skip)", "let (skip, join)");
        build(&fixture(&source, "invalid"))
            .expect("an adjacent partial continuation may feed the wider ready merge");
    }

    /// Two selections can only decide one block while both are open, which the
    /// branch rule rejects at the second one.
    #[test]
    fn rejects_a_second_selection_inside_open_branches() {
        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/independent_questions_decide_one_block.rs"
        );
        assert_eq!(
            message(&fixture(source, "invalid")),
            branch_placement("question", "Left enabled?")
        );
    }

    #[test]
    fn a_shared_continuation_may_have_two_independent_entries() {
        assert_groups(
            &fixture!("wire/behavior", "independent_entry_blocks"),
            &[group(0, &[0, 1], &[3, 4, 5], &[3, 4])],
        );
    }

    #[test]
    fn one_choice_may_own_two_disjoint_groups_around_independent_cases() {
        let function: ItemFn = parse_quote! {
            fn route(value: u8) -> u8 {
                #[choice("Which group?")]
                #[case("Independent before both groups")]
                #[case("First of the left group")]
                #[case("Second of the left group")]
                #[case("Independent between the groups")]
                #[case("First of the right group")]
                #[case("Second of the right group")]
                let (first, a, b, between, c, d) = |value| {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        4 => (),
                        _ => (),
                    }
                };

                #[action("Produce the first independent result")]
                let result = |first| { 0 };

                #[action("Build the left value from a")]
                let left = |a| { 1 };

                #[action("Build the left value from b")]
                let left = |b| { 2 };

                #[action("Produce the independent result between the groups")]
                let result = |between| { 3 };

                #[action("Build the right value from c")]
                let right = |c| { 4 };

                #[action("Build the right value from d")]
                let right = |d| { 5 };

                #[action("Use the left value")]
                let result = |left| { left };

                #[action("Use the right value")]
                let result = |right| { right };

                |result| return result;
            }
        };

        assert_groups(
            &function,
            &[
                group(0, &[0, 1, 2, 3, 4, 5], &[9], &[9]),
                group(0, &[1, 2], &[7], &[7]),
                group(0, &[4, 5], &[8], &[8]),
            ],
        );
    }

    #[test]
    fn nested_questions_each_own_a_group_over_the_shared_consumer() {
        assert_groups(
            &fixture!("wire/behavior", "nested_convergence"),
            &[
                group(0, &[0, 1], &[5, 6], &[5]),
                group(1, &[0, 1], &[5, 6], &[5]),
            ],
        );
    }

    #[test]
    fn branches_of_unequal_depth_have_one_shared_entry() {
        assert_groups(
            &fixture!("wire/behavior", "uneven_depth"),
            &[group(0, &[0, 1], &[4, 5], &[4])],
        );
    }

    /// The selected-value merge closes the outer question before its end value
    /// joins the early branch at the single return.
    #[test]
    fn nested_questions_may_share_one_result_merge() {
        assert_groups(
            &fixture!("wire/behavior", "effect_before_a_nested_terminal_branch"),
            &[
                group(1, &[0, 1], &[6, 7], &[6]),
                group(2, &[0, 1], &[7], &[7]),
            ],
        );
    }

    /// The setup block feeds both branches and the shared consumer, but
    /// nothing it does depends on the question, so it stays outside the group.
    #[test]
    fn a_setup_block_stays_outside_the_continuation_it_feeds() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, seed: u32) -> u32 {
                #[action("Prepare a shared value")]
                let prepared = |seed| { seed };

                #[question("Which way?")]
                let (yes, no) = |condition| { condition };

                #[action("Use it on the yes branch")]
                let selected = |yes, &prepared| { *prepared };

                #[action("Use it on the no branch")]
                let selected = |no, &prepared| { *prepared + 1 };

                #[action("Combine the selected and prepared values")]
                let result = |selected, prepared| { selected + prepared };
                |result| return result;
            }
        };

        assert_groups(&function, &[group(1, &[0, 1], &[4, 5], &[4])]);
    }

    #[test]
    fn a_shared_block_preceded_by_another_is_not_an_entry() {
        assert_groups(
            &fixture!("wire/behavior", "staged_convergence"),
            &[group(0, &[0, 1], &[3, 4], &[3])],
        );
    }

    #[test]
    fn alternative_producers_captured_by_return_form_a_group() {
        assert_groups(
            &fixture!("question/behavior", "run_question"),
            &[group(0, &[0, 1], &[3], &[3])],
        );
    }

    #[test]
    fn a_terminal_case_stays_outside_the_group_it_follows() {
        assert_groups(
            &fixture!("wire/behavior", "convergence_before_a_terminal_case"),
            &[
                group(0, &[0, 1], &[4], &[4]),
                group(0, &[0, 1, 2], &[5], &[5]),
            ],
        );
    }

    #[test]
    fn gates_alternative_producers_that_no_binding_unifies() {
        let unified: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                let (yes, no) = |condition| { condition };

                #[action("Build the yes value")]
                let (selected, _tag) = |yes| { (1, 1u8) };

                #[action("Build the no value")]
                let (selected, _tag) = |no| { (2, 2) };

                |selected| return selected;
            }
        };
        let ExecutionPlan::End { gates, .. } =
            build(&unified).expect("the flow is valid").execution_plan
        else {
            panic!("the plan is rooted at end")
        };
        assert!(gates.is_empty(), "the join also unifies the unused wire");

        let terminal: ItemFn = parse_quote! {
            fn choose(condition: bool) {
                #[question("Choose a value")]
                let (yes, no) = |condition| { condition };

                #[action("Build the yes value")]
                let (result, _tag) = |yes| { ((), 1u8) };

                |result| return result;

                #[action("Build the no tag before diverging")]
                let (_tag, repeat) = |no| { (2u8, ()) };

                #[cycle("Keep the other route open")]
                |repeat| {};
            }
        };
        let ExecutionPlan::End { gates, .. } =
            build(&terminal).expect("the flow is valid").execution_plan
        else {
            panic!("the plan is rooted at end")
        };
        assert_eq!(gates, ["_tag"]);
    }

    #[test]
    fn rejects_crossing_convergence_groups() {
        let source =
            include_str!("../../kaalang/tests/wire/compile_fail/overlapping_convergence_groups.rs");
        assert_eq!(
            message(&fixture(source, "invalid")),
            "kaalang choice convergence groups must be disjoint or nested"
        );
    }

    #[test]
    fn rejects_branch_local_work_after_its_merge() {
        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/block_after_convergence_in_one_branch.rs"
        );
        assert_eq!(
            message(&fixture(source, "invalid")),
            "this kaalang block must finish before the `selected` wire merge, but it waits for a value from after that merge"
        );
    }

    /// The question belongs to the wider group, so it is outside the partial
    /// group's continuation, yet every execution reaching that entry block
    /// reaches the question too.
    #[test]
    fn a_question_of_a_wider_group_may_follow_a_partial_merge() {
        assert_groups(
            &fixture!("wire/behavior", "question_after_a_partial_merge"),
            &[
                group(0, &[0, 1], &[3], &[3]),
                group(0, &[0, 1, 2], &[5, 6, 7, 8], &[5]),
                group(5, &[0, 1], &[8], &[8]),
            ],
        );
    }

    /// The question captures the first entry's output and implicitly waits for
    /// the second entry too, because its selected blocks consume that output.
    #[test]
    fn a_question_of_the_shared_continuation_may_follow_one_of_two_entries() {
        assert_groups(
            &fixture!("wire/behavior", "question_after_one_entry_block"),
            &[
                group(0, &[0, 1], &[3, 4, 5, 6, 7, 8], &[3, 4]),
                group(5, &[0, 1], &[8], &[8]),
            ],
        );
    }

    /// The reporting question follows the merges implicitly and becomes the
    /// shared continuation's entry. The counted amount remains ordinary data,
    /// so the quiet branch may leave it uncaptured.
    #[test]
    fn a_branch_may_capture_a_merged_value_of_a_shared_continuation() {
        assert_groups(
            &fixture!("wire/behavior", "a_branch_captures_a_merged_value"),
            &[
                group(0, &[0, 1], &[4, 5, 6], &[4, 5]),
                group(3, &[0, 1], &[6], &[6]),
            ],
        );
    }

    /// The note is produced in two branches, so it merges, and the block that
    /// captures it is itself branch-local. A nested question does not change
    /// that: the local value still needs a name of its own.
    #[test]
    fn rejects_a_nested_branch_local_capture_of_a_merged_wire() {
        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/nested_local_capture_of_a_merged_wire.rs"
        );
        assert_eq!(
            message(&fixture(source, "invalid")),
            "a kaalang wire with alternative producers merges before every capture; a branch-local value needs its own name"
        );
    }

    /// Every kaalang program an RFC prints is one a reader copies, so each one
    /// still has to parse. A rule the RFCs tighten reaches the examples of
    /// every section, not only the section that states it, and most examples
    /// are loose block statements rather than whole flows.
    #[test]
    fn every_program_an_rfc_prints_still_parses() {
        const RFCS: [(&str, &str); 4] = [
            ("0001", include_str!("../../../docs/rfcs/0001-language.md")),
            (
                "0002",
                include_str!("../../../docs/rfcs/0002-visual-language.md"),
            ),
            (
                "0003",
                include_str!("../../../docs/rfcs/0003-svg-renderer.md"),
            ),
            (
                "0004",
                include_str!("../../../docs/rfcs/0004-rust-lowering.md"),
            ),
        ];
        let mut checked = 0;
        for (rfc, text) in RFCS {
            for (offset, block) in fenced_rust(text) {
                let line = text[..offset].lines().count();
                for program in flow_programs(block) {
                    let function: ItemFn = syn::parse_str(&program)
                        .unwrap_or_else(|error| panic!("RFC {rfc} line {line}: {error}"));
                    if let Err(error) = crate::parse::flow(&function) {
                        panic!("RFC {rfc} line {line}: {error}");
                    }
                    checked += 1;
                }
            }
        }
        assert!(
            checked >= 28,
            "expected the RFCs to print kaalang programs, saw {checked}"
        );
    }

    /// One fence as functions the parser can take: every flow it declares, free
    /// or associated, or a loose run of block statements wrapped in one.
    fn flow_programs(block: &str) -> Vec<String> {
        if block.contains("#[kaalang]") {
            let file = syn::parse_file(block).expect("an RFC prints valid Rust");
            return file
                .items
                .iter()
                .flat_map(|item| match item {
                    syn::Item::Fn(function) => vec![function.to_token_stream()],
                    syn::Item::Impl(block) => block
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            syn::ImplItem::Fn(method) => Some(method.to_token_stream()),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .map(|function| function.to_string())
                .filter(|function| function.contains("kaalang"))
                .collect();
        }
        let Some(first) = block.lines().find(|line| !line.trim().is_empty()) else {
            return Vec::new();
        };
        let first = first.trim();
        if ["action", "call", "question", "choice", "cycle"]
            .iter()
            .any(|kind| first.starts_with(&format!("#[{kind}(")) || first == format!("#[{kind}]"))
        {
            vec![format!("fn probe() {{ {block} }}")]
        } else {
            Vec::new()
        }
    }

    /// Each fenced Rust block in one document, with its byte offset.
    fn fenced_rust(text: &str) -> Vec<(usize, &str)> {
        const FENCE: &str = "```rust\n";
        let mut blocks = Vec::new();
        let mut rest = text;
        let mut offset = 0;
        while let Some(start) = rest.find(FENCE) {
            let body = &rest[start + FENCE.len()..];
            let Some(end) = body.find("```") else { break };
            blocks.push((offset + start, &body[..end]));
            offset += start + FENCE.len() + end;
            rest = &body[end..];
        }
        blocks
    }
}
