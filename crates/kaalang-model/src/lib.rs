//! Parses and validates kaalang flows into the semantic model.

use syn::{ItemFn, Result};

mod analyze;
mod choice;
mod model;
mod parse;
mod plan;
mod resolve;

pub use choice::{choice_match, is_todo_body};
pub use model::{
    Block, BlockKind, Branch, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup,
    Execution, ExecutionPlan, Flow, Input, Join, JoinTarget, ProducerId, SemanticModel, WireMerge,
};

/// Builds the validated semantic model for one kaalang flow function.
///
/// # Errors
///
/// Returns the first violation found while parsing block syntax, resolving
/// wires to their producers, walking every possible execution, or deriving
/// convergence groups, spanned at the offending token so callers can report it
/// against the authored source.
pub fn build(function: &ItemFn) -> Result<SemanticModel> {
    let flow = parse::flow(function)?;
    resolve::flow(&flow)?;
    let (executions, convergence_groups, merges) = analyze::flow(&flow)?;
    let execution_plan = plan::flow(&flow, &executions, &merges);

    Ok(SemanticModel {
        name: function.sig.ident.clone(),
        parameters: function.sig.inputs.iter().cloned().collect(),
        return_type: function.sig.output.clone(),
        flow,
        execution_plan,
        executions,
        convergence_groups,
        merges,
    })
}

#[cfg(test)]
mod tests {
    use syn::{FnArg, ItemFn, Pat, ReturnType, Type, parse_quote};

    use super::{
        BlockKind, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup, ExecutionPlan,
        ProducerId, SemanticModel, build,
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
        let mut bodies = vec![0; model.flow.blocks.len()];
        crate::plan::verify::count(&model.execution_plan, &mut bodies);
        if let ExecutionPlan::End { index, .. } = &model.execution_plan {
            bodies[*index] += 1;
        }
        bodies[target]
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
    fn a_flow_input_named_result_needs_no_computational_block() {
        let function: ItemFn = parse_quote! {
            fn identity(result: u8) -> u8 {}
        };

        let model = build(&function).expect("the zero-computation flow is valid");
        assert_eq!(model.flow.flow_inputs, ["result"]);
        assert_eq!(model.flow.blocks.len(), 1);
        assert_eq!(model.executions.len(), 1);
        assert!(model.executions[0].blocks.is_empty());
        assert_eq!(model.executions[0].dependencies.len(), 1);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::EndArrival { result } if result == "result"
        ));
    }

    #[test]
    fn a_flow_that_produces_no_result_is_rejected() {
        assert_eq!(
            message(&parse_quote! {
                fn nothing() {}
            }),
            "a kaalang flow must produce its `result` wire"
        );
    }

    #[test]
    fn wildcard_is_not_a_flow_input_but_underscore_name_is() {
        let function: ItemFn = parse_quote! {
            fn discard(_: u8, _value: u8) {
                #[action("Finish without the flow inputs.")]
                || -> result {};
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
                |input| -> (left, right) {
                    match input {
                        0 => input,
                        _ => input,
                    }
                };

                #[action("Use the first path")]
                |left| -> result { left };

                #[action("Use the second path")]
                |right| -> result { right };
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
            } if branches.len() == 2 && joins.is_empty()
        ));
    }

    #[test]
    fn records_alternative_producers_against_each_execution() {
        let function: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                |condition| -> (yes, no) { condition };

                #[action("Build the yes value")]
                |yes| -> selected { 1 };

                #[action("Build the no value")]
                |no| -> selected { 2 };

                #[action("Use the selected value")]
                |selected| -> result { selected };
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
        assert!(yes.dependencies.contains(&CaptureDependency {
            producer: ProducerId::BlockOutput {
                block: 3,
                output: 0
            },
            capture: CaptureId { block: 4, input: 0 },
        }));
    }

    #[test]
    fn records_one_shared_consumer_and_orders_join_wires_by_producer() {
        let function: ItemFn = parse_quote! {
            fn choose(condition: bool) -> (u32, u32) {
                #[question("Choose values")]
                |condition| -> (yes, no) { condition };

                #[action("Build the yes values")]
                |yes| -> (first, second) { (1, 2) };

                #[action("Build the no values")]
                |no| -> (first, second) { (3, 4) };

                #[action("Use the selected values")]
                |second, first| -> result { (first, second) };
            }
        };

        let model = build(&function).expect("the alternative producers are valid");
        let ExecutionPlan::Question {
            join: Some(join), ..
        } = end_body(&model.execution_plan)
        else {
            panic!("the question must record its join")
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
                |&input| -> first { *input };

                #[action("Produce the second result")]
                |&input| -> second { *input + 1 };

                #[action("Pair the two results")]
                |first, second| -> result { (first, second) };
            }
        };

        let model = build(&function).expect("independent borrowers are valid");
        assert_eq!(model.executions.len(), 1);
        assert_eq!(model.executions[0].blocks, [0, 1, 2]);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Action { index: 0, next }
                if matches!(next.as_ref(), ExecutionPlan::Action { index: 1, next }
                    if matches!(next.as_ref(), ExecutionPlan::Action { index: 2, next }
                        if matches!(next.as_ref(), ExecutionPlan::EndArrival { .. })))
        ));
    }

    /// The second question opens its branches only once the first question's
    /// branches have merged, so both bodies are still emitted once.
    #[test]
    fn successive_questions_share_every_body() {
        let function: ItemFn = parse_quote! {
            fn route(left: bool, right: bool) -> u8 {
                #[question("Choose the right value")]
                |right| -> (x, y) { right };
                #[action("Build the first right value")]
                |x| -> value { 10u8 };
                #[action("Build the second right value")]
                |y| -> value { 20u8 };
                #[question("Choose the left path")]
                |left| -> (a, b) { left };
                #[action("Use the left path")]
                |a, value| -> result { value + 1 };
                #[action("Use the other left path")]
                |b, value| -> result { value + 2 };
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
                |seed| -> prepared { seed };

                #[question("Which way?")]
                |condition| -> (yes, no) { condition };

                #[action("Use it on the yes branch")]
                |yes, &prepared| -> result { *prepared };

                #[action("Use it on the no branch")]
                |no, prepared| -> result { prepared + 1 };
            }
        };

        let model = build(&function).expect("the flow is valid");
        assert_eq!(model.executions.len(), 2);
        assert!(matches!(
            end_body(&model.execution_plan),
            ExecutionPlan::Action { index: 0, next }
                if matches!(next.as_ref(), ExecutionPlan::Question { index: 1, join: None, .. })
        ));
    }

    #[test]
    fn records_nested_join_and_terminal_case_topology() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, value: usize) -> usize {
                #[question("Take the branching path?")]
                |condition, &value| -> (yes, no) { condition };

                #[choice("Which branch?")]
                #[case("First")]
                #[case("Second")]
                #[case("Terminal")]
                |yes, value| -> (first, second, third) {
                    match value {
                        0 => (),
                        1 => (),
                        _ => (),
                    }
                };

                #[action("Build the first value")]
                |first| -> selected { 1 };

                #[action("Build the second value")]
                |second| -> selected { 2 };

                #[action("Produce the terminal result")]
                |third| -> result { 3 };

                #[action("Produce the selected result")]
                |selected| -> result { selected };

                #[action("Produce the no-branch result")]
                |no| -> result { 0 };
            }
        };

        let model = build(&function).expect("the flow is valid");
        assert_eq!(model.executions.len(), 4);
        let ExecutionPlan::Question {
            index,
            branches: [yes, no],
            join: None,
        } = end_body(&model.execution_plan)
        else {
            panic!("the root must be a question without a join")
        };
        assert_eq!(*index, 0);
        assert!(matches!(
            no.plan.as_ref(),
            ExecutionPlan::Action { index: 6, .. }
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
                if matches!(next.as_ref(), ExecutionPlan::EndArrival { .. })
        ));
        let [join] = joins.as_slice() else {
            panic!("the first two cases share one join")
        };
        assert_eq!(join.branches, [0, 1]);
        assert_eq!(join.wires, ["selected"]);
        assert!(matches!(
            join.next.as_ref(),
            ExecutionPlan::Action { index: 5, next }
                if matches!(next.as_ref(), ExecutionPlan::EndArrival { .. })
        ));
        assert_eq!(count_block(&model, 7), 1);
    }

    #[test]
    fn rejects_a_branch_output_captured_twice() {
        let source =
            include_str!("../../kaalang/tests/wire/compile_fail/branch_output_captured_twice.rs");
        assert_eq!(
            message(&fixture(source, "invalid")),
            "a kaalang branch output is captured by at most one block"
        );
    }

    /// The second question opens its own branches while the first question's
    /// are still separate, which is reported before the three executions that
    /// would leave the flow without its `result` wire.
    #[test]
    fn rejects_a_conjunction_of_independent_branch_outputs() {
        let function: ItemFn = parse_quote! {
            fn both(left: bool, right: bool) -> u8 {
                #[question("Left?")]
                |left| -> (a, _b) { left };
                #[question("Right?")]
                |right| -> (c, _d) { right };
                #[action("Both")]
                |a, c| -> result { 1u8 };
            }
        };

        assert_eq!(message(&function), branch_placement("question", "Left?"));
    }

    #[test]
    fn rejects_a_borrowed_branch_output() {
        let question =
            include_str!("../../kaalang/tests/question/compile_fail/borrowed_question_output.rs");
        assert_eq!(
            message(&fixture(question, "invalid")),
            "a kaalang branch output is consumed, never borrowed"
        );
        let choice =
            include_str!("../../kaalang/tests/choice/compile_fail/borrowed_choice_output.rs");
        assert_eq!(
            message(&fixture(choice, "invalid")),
            "a kaalang branch output is consumed, never borrowed"
        );
    }

    #[test]
    fn rejects_branch_local_access_to_a_name_with_alternative_producers() {
        let function: ItemFn = parse_quote! {
            fn route(outer: bool, inner: bool) -> u8 {
                #[question("Choose the source")]
                |outer| -> (yes, no) { outer };

                #[action("Prepare the nested branch")]
                |no| -> (trigger, branch_gate) { ((), ()) };

                #[question("Choose the control output")]
                |trigger, inner| -> (shared, skip) { inner };

                #[action("Produce borrowable data")]
                |yes| -> (shared, borrow_gate) { ((), ()) };

                #[action("Consume the nested control")]
                |shared, branch_gate| -> result { 1u8 };

                #[action("Consume the other nested control")]
                |skip, branch_gate| -> result { 2u8 };

                #[action("Borrow only the action output")]
                |&shared, borrow_gate| -> result { 3u8 };
            }
        };

        assert_eq!(
            message(&function),
            "a kaalang wire with alternative producers merges before every capture; a branch-local value needs its own name"
        );
    }

    #[test]
    fn one_choice_may_own_two_joins() {
        let function: ItemFn = parse_quote! {
            fn route(value: u8) -> u8 {
                #[choice("Which group?")]
                #[case("First of the left group")]
                #[case("Second of the left group")]
                #[case("Terminal")]
                #[case("First of the right group")]
                #[case("Second of the right group")]
                |value| -> (a, b, done, c, d) {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        _ => (),
                    }
                };

                #[action("Build the left value from a")]
                |a| -> left { 1 };

                #[action("Build the left value from b")]
                |b| -> left { 2 };

                #[action("Produce the terminal result")]
                |done| -> result { 3 };

                #[action("Build the right value from c")]
                |c| -> right { 4 };

                #[action("Build the right value from d")]
                |d| -> right { 5 };

                #[action("Use the left value")]
                |left| -> result { left };

                #[action("Use the right value")]
                |right| -> result { right };
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
                if matches!(next.as_ref(), ExecutionPlan::EndArrival { .. })
        ));
        let [left, right] = joins.as_slice() else {
            panic!("the choice owns two joins")
        };
        assert_eq!(left.branches, [0, 1]);
        assert_eq!(left.wires, ["left"]);
        assert_eq!(right.branches, [3, 4]);
        assert_eq!(right.wires, ["right"]);
        assert!(matches!(
            left.next.as_ref(),
            ExecutionPlan::Action { index: 6, .. }
        ));
        assert!(matches!(
            right.next.as_ref(),
            ExecutionPlan::Action { index: 7, .. }
        ));
        for block in 0..8 {
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
    fn an_outside_nested_branch_may_finish_but_not_rejoin_an_ordinary_continuation() {
        build(&fixture!(
            "wire/behavior",
            "nested_branch_passes_a_question_join"
        ))
        .expect("the early result is outside the shared merge");

        let source = include_str!(
            "../../kaalang/tests/wire/compile_fail/nested_branch_passes_a_case_join.rs"
        )
        .replace("(join, skip) { inner }", "(skip, join) { !inner }");
        assert_eq!(
            message(&fixture(&source, "invalid")),
            "a nested kaalang branch cannot bypass the `shared` wire merge and rejoin at `ready`; merge before or with the enclosing branches"
        );
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
    fn one_choice_may_own_two_disjoint_groups_around_terminal_cases() {
        let function: ItemFn = parse_quote! {
            fn route(value: u8) -> u8 {
                #[choice("Which group?")]
                #[case("Terminal before both groups")]
                #[case("First of the left group")]
                #[case("Second of the left group")]
                #[case("Terminal between the groups")]
                #[case("First of the right group")]
                #[case("Second of the right group")]
                |value| -> (first, a, b, between, c, d) {
                    match value {
                        0 => (),
                        1 => (),
                        2 => (),
                        3 => (),
                        4 => (),
                        _ => (),
                    }
                };

                #[action("Produce the first terminal result")]
                |first| -> result { 0 };

                #[action("Build the left value from a")]
                |a| -> left { 1 };

                #[action("Build the left value from b")]
                |b| -> left { 2 };

                #[action("Produce the terminal result between the groups")]
                |between| -> result { 3 };

                #[action("Build the right value from c")]
                |c| -> right { 4 };

                #[action("Build the right value from d")]
                |d| -> right { 5 };

                #[action("Use the left value")]
                |left| -> result { left };

                #[action("Use the right value")]
                |right| -> result { right };
            }
        };

        assert_groups(
            &function,
            &[group(0, &[1, 2], &[7], &[7]), group(0, &[4, 5], &[8], &[8])],
        );
    }

    #[test]
    fn nested_questions_each_own_a_group_over_the_shared_consumer() {
        assert_groups(
            &fixture!("wire/behavior", "nested_convergence"),
            &[group(0, &[0, 1], &[5], &[5]), group(1, &[0, 1], &[5], &[5])],
        );
    }

    #[test]
    fn branches_of_unequal_depth_have_one_shared_entry() {
        assert_groups(
            &fixture!("wire/behavior", "uneven_depth"),
            &[group(0, &[0, 1], &[4], &[4])],
        );
    }

    /// The nested question's terminal branch and the preceding effect stay
    /// outside the outer group; the nested question itself forms no group
    /// because only its late branch reaches the shared consumer.
    #[test]
    fn a_continuing_branch_may_hold_a_nested_terminal_branch() {
        assert_groups(
            &fixture!("wire/behavior", "effect_before_a_nested_terminal_branch"),
            &[group(1, &[0, 1], &[6], &[6])],
        );
    }

    /// The setup block feeds both branches and the shared consumer, but
    /// nothing it does depends on the question, so it stays outside the group.
    #[test]
    fn a_setup_block_stays_outside_the_continuation_it_feeds() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, seed: u32) -> u32 {
                #[action("Prepare a shared value")]
                |seed| -> prepared { seed };

                #[question("Which way?")]
                |condition| -> (yes, no) { condition };

                #[action("Use it on the yes branch")]
                |yes, &prepared| -> selected { *prepared };

                #[action("Use it on the no branch")]
                |no, &prepared| -> selected { *prepared + 1 };

                #[action("Combine the selected and prepared values")]
                |selected, prepared| -> result { selected + prepared };
            }
        };

        assert_groups(&function, &[group(1, &[0, 1], &[4], &[4])]);
    }

    #[test]
    fn a_shared_block_preceded_by_another_is_not_an_entry() {
        assert_groups(
            &fixture!("wire/behavior", "staged_convergence"),
            &[group(0, &[0, 1], &[3, 4], &[3])],
        );
    }

    #[test]
    fn alternative_producers_captured_only_by_end_form_no_group() {
        assert_groups(&fixture!("question/behavior", "run_question"), &[]);
    }

    #[test]
    fn a_terminal_case_stays_outside_the_group_it_follows() {
        assert_groups(
            &fixture!("wire/behavior", "convergence_before_a_terminal_case"),
            &[group(0, &[0, 1], &[4], &[4])],
        );
    }

    #[test]
    fn gates_alternative_producers_that_no_binding_unifies() {
        let unified: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                |condition| -> (yes, no) { condition };

                #[action("Build the yes value")]
                |yes| -> (selected, _tag) { (1, 1u8) };

                #[action("Build the no value")]
                |no| -> (selected, _tag) { (2, 2u8) };

                #[action("Use the selected value")]
                |selected| -> result { selected };
            }
        };
        let ExecutionPlan::End { gates, .. } =
            build(&unified).expect("the flow is valid").execution_plan
        else {
            panic!("the plan is rooted at end")
        };
        assert!(gates.is_empty(), "the join also unifies the unused wire");

        let terminal: ItemFn = parse_quote! {
            fn choose(condition: bool) -> u32 {
                #[question("Choose a value")]
                |condition| -> (yes, no) { condition };

                #[action("Build the yes value")]
                |yes| -> (selected, _tag) { (1, 1u8) };

                #[action("Build the no result")]
                |no| -> (result, _tag) { (2, 2u8) };

                #[action("Use the selected value")]
                |selected| -> result { selected };
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
                group(0, &[0, 1, 2], &[5, 6, 7], &[5]),
            ],
        );
    }

    /// The question captures the first entry's output and implicitly waits for
    /// the second entry too, because its selected blocks consume that output.
    #[test]
    fn a_question_of_the_shared_continuation_may_follow_one_of_two_entries() {
        assert_groups(
            &fixture!("wire/behavior", "question_after_one_entry_block"),
            &[group(0, &[0, 1], &[3, 4, 5, 6, 7], &[3, 4])],
        );
    }

    /// The reporting question follows the merges implicitly and becomes the
    /// shared continuation's entry. The counted amount remains ordinary data,
    /// so the quiet branch may leave it uncaptured.
    #[test]
    fn a_branch_may_capture_a_merged_value_of_a_shared_continuation() {
        assert_groups(
            &fixture!("wire/behavior", "a_branch_captures_a_merged_value"),
            &[group(0, &[0, 1], &[4, 5], &[4, 5])],
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
}
