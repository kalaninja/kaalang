//! Enters one cycle iteration; reaching its end records a repeat.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::Error;

use super::{LoopState, State, Walk};
use crate::model::{Flow, ProducerId};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let bindings = walk.flow.blocks[block]
        .inputs
        .iter()
        .enumerate()
        .map(|(input, declaration)| {
            (
                declaration
                    .binding
                    .clone()
                    .expect("a cycle capture declares a local binding"),
                ProducerId::CycleInput { block, input },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let outside = LoopState {
        available: std::mem::replace(&mut state.available, bindings.clone()),
        produced: std::mem::replace(
            &mut state.produced,
            bindings.keys().cloned().collect::<BTreeSet<Ident>>(),
        ),
    };
    state.loops.insert(block, outside);
    walk.visit(block + 1, state);
}

/// Every completing cycle output needs a consumer in some execution.
pub(super) fn uncaptured(output: &Ident) -> Error {
    Error::new(
        output.span(),
        "every kaalang cycle output must have a consumer",
    )
}

/// A selection stops governing its iteration's branches when that cycle completes.
/// Reaching a later block depends on completing the cycle. This is control
/// order, not a capture dependency or a wire merge.
pub(super) fn closed_before(flow: &Flow, selection: usize, next: usize) -> bool {
    std::iter::once(selection)
        .chain(flow.enclosing(selection))
        .any(|index| flow.blocks[index].loop_end.is_some_and(|end| end <= next))
}

#[cfg(test)]
mod tests {
    #[test]
    fn moving_the_repeating_case_to_either_edge_is_valid() {
        let source = include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat.rs");
        for outputs in ["(advance, first, last)", "(first, last, advance)"] {
            let source = source.replace("(first, advance, last)", outputs);
            crate::build(&crate::tests::fixture(&source, "invalid"))
                .expect("adjacent exits leave the return an outer contour");
        }
        let source = r#"
            #[kaalang]
            fn invalid(mode: u8) -> u8 {
                #[cycle("Select an exit from a nested cycle.")]
                let selected = |mut mode| {
                    #[cycle("Advance at most once.")]
                    let inner = |mut mode| {
                        #[question("Exit immediately?")]
                        let (done, check) = |mode| mode == 0;

                        #[question("Advance once?")]
                        let (done, advance) = |check, mode| mode == 1;

                        #[action("Advance to the final case.")]
                        |advance, &mut mode| *mode = 2;

                        |done, mode| break mode;
                    };

                    |inner| break inner;
                };

                |selected| return selected;
            }
        "#;
        crate::build(&crate::tests::fixture(source, "invalid"))
            .expect("nested questions can place their exits next to each other");
    }

    #[test]
    fn a_converged_selection_does_not_split_loop_exit_routes() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/multiple_exits.rs")
            .replace(
                "    let selected = |mut mode| {",
                r#"    let selected = |mut mode| {
        #[question("Prepare this iteration?")]
        let (left, right) = |mode| mode == 0;

        #[action("Prepare the left branch.")]
        let ready = |left| ();

        #[action("Prepare the right branch.")]
        let ready = |right| ();

        #[action("Finish the shared preparation.")]
        |ready| {};
"#,
            );
        crate::build(&crate::tests::fixture(&source, "multiple_exits"))
            .expect("completed branches do not duplicate the following exit routes");
    }

    #[test]
    fn a_partial_merge_before_loop_entry_does_not_split_its_exit_routes() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/conditional_entry.rs")
            .replace(
                "#[question(\"Run the counter?\")]\n    let (run, skip) = |enabled| enabled;",
                r#"#[choice("Run the counter?")]
    #[case("Run when enabled.")]
    #[case("Run for a zero limit.")]
    #[case("Skip the counter.")]
    let (first, second, skip) = |enabled, limit| match (enabled, limit) {
        (true, _) => (),
        (false, 0) => (),
        _ => (),
    };

    #[action("Run when enabled.")]
    let run = |first| ();

    #[action("Run for a zero limit.")]
    let run = |second| ();"#,
            );
        crate::build(&crate::tests::fixture(&source, "conditional_entry"))
            .expect("routes skipping the loop do not decide its exit order");
    }
}
