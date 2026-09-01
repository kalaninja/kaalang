//! Parses and validates Contour flows into the semantic model.

use syn::{ItemFn, Result};

mod analyze;
mod choice;
mod model;
mod parse;
mod resolve;

pub use choice::{choice_match, is_todo_body};
pub use model::{Block, BlockKind, Branch, Convergence, Flow, Graph, Input, Merge, Plan};

/// Builds the validated semantic model for one Contour flow function.
///
/// # Errors
///
/// Returns the first violation found while parsing block syntax, resolving
/// wires to their producers, or walking every path, spanned at the offending
/// token so callers can report it against the authored source.
pub fn build(function: &ItemFn) -> Result<Graph> {
    let flow = parse::flow(function)?;
    resolve::flow(&flow)?;
    let plan = analyze::flow(&flow)?;

    Ok(Graph {
        name: function.sig.ident.clone(),
        parameters: function.sig.inputs.iter().cloned().collect(),
        return_type: function.sig.output.clone(),
        flow,
        plan,
    })
}

#[cfg(test)]
mod tests {
    use syn::{FnArg, ItemFn, Pat, ReturnType, Type, parse_quote};

    use super::{BlockKind, Plan, build};

    fn count_block(plan: &Plan, target: usize) -> usize {
        match plan {
            Plan::Action { index, next } => {
                usize::from(*index == target) + count_block(next, target)
            }
            Plan::Question {
                index,
                branches,
                merge,
                convergence,
            } => {
                usize::from(*index == target)
                    + branches
                        .iter()
                        .map(|branch| count_block(&branch.plan, target))
                        .sum::<usize>()
                    + merge
                        .as_ref()
                        .map_or(0, |merge| count_block(&merge.next, target))
                    + convergence
                        .as_ref()
                        .map_or(0, |convergence| count_block(&convergence.next, target))
            }
            Plan::Choice {
                index,
                branches,
                merge,
                convergence,
            } => {
                usize::from(*index == target)
                    + branches
                        .iter()
                        .map(|branch| count_block(&branch.plan, target))
                        .sum::<usize>()
                    + merge
                        .as_ref()
                        .map_or(0, |merge| count_block(&merge.next, target))
                    + convergence
                        .as_ref()
                        .map_or(0, |convergence| count_block(&convergence.next, target))
            }
            Plan::End { index, body } => usize::from(*index == target) + count_block(body, target),
            Plan::EndArrival { .. } | Plan::Arrival { .. } | Plan::Yield { .. } => 0,
        }
    }

    fn end_body(plan: &Plan) -> &Plan {
        let Plan::End { body, .. } = plan else {
            panic!("the verified plan must be rooted at End")
        };
        body
    }

    #[test]
    fn zero_wire_flow_has_no_implicit_unit_wire() {
        let function: ItemFn = parse_quote! {
            fn nothing() {
                #[end]
                || {};
            }
        };

        let graph = build(&function).expect("the zero-wire flow is valid");
        assert!(graph.flow.sources.is_empty());
        assert_eq!(graph.flow.blocks.len(), 1);
        assert!(graph.flow.blocks[0].inputs.is_empty());
        assert!(graph.flow.blocks[0].outputs.is_empty());
        assert!(matches!(
            end_body(&graph.plan),
            Plan::EndArrival { inputs } if inputs.is_empty()
        ));
    }

    #[test]
    fn wildcard_is_not_a_source_but_underscore_name_is() {
        let function: ItemFn = parse_quote! {
            fn discard(_: u8, _value: u8) {
                #[end]
                || {};
            }
        };

        let graph = build(&function).expect("both ignored parameter forms are valid");
        assert_eq!(graph.flow.sources.len(), 1);
        assert_eq!(graph.flow.sources[0], "_value");
        assert!(matches!(
            graph.parameters.as_slice(),
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

                #[end]
                |result| {};
            }
        };

        let graph = build(&function).expect("the flow is valid");
        let choice = &graph.flow.blocks[0];

        assert_eq!(graph.name, "choose");
        assert_eq!(graph.parameters.len(), 1);
        let FnArg::Typed(parameter) = &graph.parameters[0] else {
            panic!("the parameter must be typed")
        };
        let Pat::Ident(parameter) = parameter.pat.as_ref() else {
            panic!("the parameter must retain its authored name")
        };
        assert_eq!(parameter.ident, "input");
        let ReturnType::Type(_, return_type) = &graph.return_type else {
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
            end_body(&graph.plan),
            Plan::Choice {
                branches: paths,
                merge: None,
                ..
            } if paths.len() == 2
        ));
    }

    #[test]
    fn preserves_path_exclusive_producer_occurrences() {
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

                #[end]
                |result| {};
            }
        };

        let graph = build(&function).expect("the path-exclusive producers are valid");
        assert_eq!(graph.flow.blocks[1].outputs[0], "selected");
        assert_eq!(graph.flow.blocks[2].outputs[0], "selected");
        assert_eq!(graph.flow.blocks[3].inputs[0].ident, "selected");
    }

    #[test]
    fn records_one_shared_consumer_and_orders_yields_by_its_inputs() {
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

                #[end]
                |result| {};
            }
        };

        let graph = build(&function).expect("the path-exclusive producers are valid");
        let Plan::Question {
            convergence: Some(convergence),
            ..
        } = end_body(&graph.plan)
        else {
            panic!("the question must record its implicit convergence")
        };

        assert_eq!(
            convergence
                .wires
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["second", "first"]
        );
        assert_eq!(count_block(&graph.plan, 3), 1);
        assert_eq!(count_block(&graph.plan, 4), 1);
    }

    #[test]
    fn records_nested_branch_merge_and_early_end_topology() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, value: usize) -> usize {
                #[question("Take the branching path?")]
                |condition, &value| -> (yes, no) { condition };

                #[choice("Which branch?")]
                #[case("First")]
                #[case("Second")]
                #[case("End")]
                |yes, value| -> (first, second, third) {
                    match value {
                        0 => (),
                        1 => (),
                        _ => (),
                    }
                };

                #[action("Build the first value")]
                |first| -> first_value { 1 };

                #[action("Build the second value")]
                |second| -> second_value { 2 };

                #[action("Finish the early End path")]
                |third| -> result { 3 };

                #[merge]
                |first_value, second_value| -> selected {};

                #[action("Return the merged value")]
                |selected| -> result { selected };

                #[action("Return from the no branch")]
                |no| -> result { 0 };

                #[end]
                |result| {};
            }
        };

        let graph = build(&function).expect("the flow is valid");
        let Plan::Question {
            index,
            branches: [yes, no],
            merge,
            ..
        } = end_body(&graph.plan)
        else {
            panic!("the root must be a question")
        };
        assert_eq!(*index, 0);
        assert!(merge.is_none());
        assert!(matches!(no.plan.as_ref(), Plan::Action { index: 7, .. }));

        let Plan::Choice {
            index,
            branches,
            merge: Some(merge),
            ..
        } = yes.plan.as_ref()
        else {
            panic!("the yes branch must contain the choice")
        };
        assert_eq!(*index, 1);
        assert_eq!(branches.len(), 3);
        assert!(!branches[0].early_return);
        assert!(!branches[1].early_return);
        assert!(branches[2].early_return);
        assert!(matches!(
            branches[0].plan.as_ref(),
            Plan::Action { index: 2, next }
                if matches!(next.as_ref(), Plan::Arrival { input, merge: 5 } if input == "first_value")
        ));
        assert!(matches!(
            branches[1].plan.as_ref(),
            Plan::Action { index: 3, next }
                if matches!(next.as_ref(), Plan::Arrival { input, merge: 5 } if input == "second_value")
        ));
        assert!(matches!(
            branches[2].plan.as_ref(),
            Plan::Action { index: 4, next }
                if matches!(next.as_ref(), Plan::EndArrival { .. })
        ));
        assert_eq!(merge.index, 5);
        assert!(matches!(
            merge.next.as_ref(),
            Plan::Action { index: 6, next }
                if matches!(next.as_ref(), Plan::EndArrival { .. })
        ));
        assert_eq!(count_block(&graph.plan, 8), 1);
    }
}
