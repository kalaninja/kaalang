//! Checks that the questions and choices deciding whether a block executes
//! form a chain of nested selections rather than independent ones.
//!
//! Branch placement rejects independent selections even after disjoint partial
//! merges. This pass checks capture ancestry and the order of normal loop exits.

use std::collections::BTreeSet;

use syn::{Error, Result};

use crate::model::{BlockKind, Flow};

use super::frame::Frames;

use super::only_difference;

/// For every computational block, the questions and choices that decide it. A
/// question or choice decides a block when two executions select different
/// branches of it, agree at every other question or choice they both run, and
/// differ in whether the block participates. A question or choice on a path
/// that one execution cut short runs in only one of the two and takes no part
/// in the comparison. An execution that repeats a cycle not enclosing the block
/// never reaches it, so it does not skip the block either. Each block compares
/// the selections its frame sees.
pub(super) fn deciders(flow: &Flow, frames: &Frames<'_>) -> Vec<BTreeSet<usize>> {
    (0..flow.blocks.len() - 1)
        .map(|block| {
            let (running, skipping): (Vec<_>, Vec<_>) = frames
                .at(block)
                .iter()
                .partition(|execution| execution.participates(block));
            running
                .iter()
                .flat_map(|run| {
                    skipping
                        .iter()
                        .filter(|skip| flow.reaches(skip, block))
                        .filter_map(|skip| only_difference(run, skip))
                })
                .collect()
        })
        .collect()
}

/// The deciders of one block are pairwise dependent: one lies in the
/// continuation of a branch of the other, or after the loop containing it exits.
/// Loop exit order does not add captures or convergence groups. Two independent
/// selections would withhold the block's inputs in a way none of its own
/// decisions explains, whether by leaving a branch output unselected or a wire
/// unproduced.
pub(super) fn flow(
    flow: &Flow,
    frames: &Frames<'_>,
    precedence: &[Vec<BTreeSet<usize>>],
) -> Result<()> {
    let executions = frames.view(None);
    let dependent = |first: usize, second: usize| {
        precedence
            .iter()
            .zip(executions)
            .any(|(preceding, execution)| {
                preceding[first].contains(&second)
                    || preceding[second].contains(&first)
                    || (execution.participates(first)
                        && execution.participates(second)
                        && (super::loop_block::closed_before(flow, first, second)
                            || super::loop_block::closed_before(flow, second, first)))
            })
    };
    for (block, deciders) in deciders(flow, frames).into_iter().enumerate() {
        let deciders = deciders.into_iter().collect::<Vec<_>>();
        let independent = deciders.iter().enumerate().any(|(position, &first)| {
            deciders[position + 1..]
                .iter()
                .any(|&second| !dependent(first, second))
        });
        if independent {
            // A boundary consumer is not authored: it carries its output's span.
            let message = if flow.blocks[block].kind == BlockKind::Export {
                format!(
                    "this kaalang cycle exports `{}` on routes decided by two independent questions, choices, or cycles",
                    super::exported_name(flow, block)
                )
            } else {
                "this kaalang block must not be decided by two independent questions, choices, or cycles".to_owned()
            };
            return Err(Error::new(flow.blocks[block].span, message));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::deciders;
    use crate::analyze::frame::Frames;
    use crate::build;

    /// The questions deciding each computational block of one fixture, in
    /// authored order.
    fn fixture(source: &str, flow: &str) -> Vec<Vec<usize>> {
        let model = build(&crate::tests::fixture(source, flow)).expect("the fixture is valid");
        deciders(
            &model.analysis.flow,
            &Frames::of(&model.analysis.flow, &model.analysis.executions),
        )
        .into_iter()
        .map(|deciders| deciders.into_iter().collect())
        .collect()
    }

    /// Outside a cycle with several outputs only its exported output decides:
    /// the body's own selections stay inside it (RFC 0006 §5.2).
    #[test]
    fn a_cycle_with_several_outputs_decides_the_blocks_after_it() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/alternative_outputs.rs");
        let deciders = fixture(source, "alternative_outputs");
        assert_eq!(deciders[7], [0], "return the found item");
        assert_eq!(deciders[8], [0], "report exhaustion");
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
                vec![],     // the return
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
                vec![],  // right enabled?
                vec![0], // provide the right value
                vec![0], // provide no right value
                vec![],  // left enabled?
                vec![3], // work with the right value
                vec![3], // skip the work
                vec![],  // return the merged result
            ]
        );
        let source = include_str!("../../../kaalang/tests/wire/behavior/captured_in_one_branch.rs");
        assert_eq!(
            fixture(source, "captured_in_one_branch"),
            [vec![], vec![0], vec![0], vec![]]
        );
    }
}
