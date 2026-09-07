//! Checks that the questions and choices deciding whether a block executes
//! form a chain of nested selections rather than independent ones.

use std::collections::BTreeSet;

use syn::{Error, Result};

use crate::model::{Execution, Flow};

use super::only_difference;

/// For every computational block, the questions and choices that decide it. A
/// question or choice decides a block when two executions select different
/// branches of it, agree at every other question or choice they both run, and
/// differ in whether the block participates. A question or choice on a path
/// that one execution cut short runs in only one of the two and takes no part
/// in the comparison.
pub(super) fn deciders(flow: &Flow, executions: &[Execution]) -> Vec<BTreeSet<usize>> {
    (0..flow.blocks.len() - 1)
        .map(|block| {
            let (running, skipping): (Vec<_>, Vec<_>) = executions
                .iter()
                .partition(|execution| execution.participates(block));
            running
                .iter()
                .flat_map(|run| {
                    skipping
                        .iter()
                        .filter_map(|skip| only_difference(run, skip))
                })
                .collect()
        })
        .collect()
}

/// The deciders of one block are pairwise dependent: one lies in the
/// continuation of a branch of the other. Two independent selections would
/// withhold the block's inputs in a way none of its own decisions explains,
/// whether by leaving a branch output unselected, a wire unproduced, or a wire
/// consumed.
pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    precedence: &[Vec<BTreeSet<usize>>],
) -> Result<()> {
    let dependent = |first: usize, second: usize| {
        precedence.iter().any(|preceding| {
            preceding[first].contains(&second) || preceding[second].contains(&first)
        })
    };
    for (block, deciders) in deciders(flow, executions).into_iter().enumerate() {
        let deciders = deciders.into_iter().collect::<Vec<_>>();
        let independent = deciders.iter().enumerate().any(|(position, &first)| {
            deciders[position + 1..]
                .iter()
                .any(|&second| !dependent(first, second))
        });
        if independent {
            return Err(Error::new(
                flow.blocks[block].span,
                "this kaalang block must not be decided by two independent questions or choices",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use syn::ItemFn;

    use super::deciders;
    use crate::build;

    /// The questions deciding each computational block of one fixture, in
    /// authored order.
    fn fixture(source: &str, flow: &str) -> Vec<Vec<usize>> {
        let file = syn::parse_file(source).expect("the fixture parses");
        let function: ItemFn = file
            .items
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == flow => Some(function),
                _ => None,
            })
            .expect("the fixture declares its flow");
        let model = build(&function).expect("the fixture is valid");
        deciders(&model.flow, &model.executions)
            .into_iter()
            .map(|deciders| deciders.into_iter().collect())
            .collect()
    }

    /// The outer question keeps deciding the late blocks although the nested
    /// question is absent from the direct execution: only the questions both
    /// executions run take part in the comparison.
    #[test]
    fn a_nested_terminal_branch_keeps_the_outer_question_as_a_decider() {
        let source = include_str!(
            "../../../kaalang/tests/wire/behavior/nested_terminal_branch_drops_a_wire.rs"
        );
        assert_eq!(
            fixture(source, "nested_terminal_branch_drops_a_wire"),
            [
                vec![],     // the outer question
                vec![0],    // prepare the nested values
                vec![0],    // the nested question
                vec![0, 2], // the early result
                vec![0, 2], // the late result with the extra value
                vec![0],    // the direct result
            ]
        );
    }

    #[test]
    fn a_converged_selection_does_not_decide_the_block_it_meets() {
        let source = include_str!(
            "../../../kaalang/tests/wire/behavior/converged_selection_meets_a_branch.rs"
        );
        assert_eq!(
            fixture(source, "converged_selection_meets_a_branch"),
            [
                vec![],  // left enabled?
                vec![],  // right enabled?
                vec![1], // provide the right value
                vec![1], // provide no right value
                vec![0], // work with the right value
                vec![0], // skip the work
            ]
        );
        let source = include_str!("../../../kaalang/tests/wire/behavior/captured_in_one_branch.rs");
        assert_eq!(
            fixture(source, "captured_in_one_branch"),
            [vec![], vec![0], vec![0]]
        );
    }
}
