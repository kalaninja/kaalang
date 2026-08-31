//! Parses and validates Contour flows into the semantic model.

use syn::{ItemFn, Result};

mod analyze;
mod choice;
mod model;
mod parse;
mod resolve;

pub use choice::{choice_match, is_todo_body};
pub use model::{Block, BlockKind, Branch, Flow, Graph, Input, Merge, Plan};

/// Builds the validated semantic model for one Contour flow function.
///
/// # Errors
///
/// Returns the first violation found while parsing block syntax, resolving
/// wires to their producers, or walking every path, spanned at the offending
/// token so callers can report it against the authored source.
pub fn build(function: &ItemFn) -> Result<Graph> {
    let mut flow = parse::flow(function)?;
    resolve::flow(&mut flow)?;
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
                |left| -> first_result { left };

                #[action("Use the second path")]
                |right| -> second_result { right };
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
            graph.plan,
            Plan::Choice {
                branches: ref paths,
                merge: None,
                ..
            } if paths.len() == 2
        ));
    }

    #[test]
    fn records_nested_branch_merge_and_terminal_sibling_topology() {
        let function: ItemFn = parse_quote! {
            fn route(condition: bool, value: usize) -> usize {
                #[question("Take the branching path?")]
                |condition| -> (yes, no) { condition };

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
                |first| -> first_value { 1 };

                #[action("Build the second value")]
                |second| -> second_value { 2 };

                #[action("Return from the terminal sibling")]
                |third| -> third_result { 3 };

                #[merge]
                |first_value, second_value| -> selected {};

                #[action("Return the merged value")]
                |selected| -> selected_result { selected };

                #[action("Return from the no branch")]
                |no| -> no_result { 0 };
            }
        };

        let graph = build(&function).expect("the flow is valid");
        let Plan::Question {
            index,
            branches: [yes, no],
            merge,
        } = &graph.plan
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
                if matches!(next.as_ref(), Plan::Terminal { .. })
        ));
        assert_eq!(merge.index, 5);
        assert!(matches!(
            merge.next.as_ref(),
            Plan::Action { index: 6, next }
                if matches!(next.as_ref(), Plan::Terminal { .. })
        ));
    }
}
