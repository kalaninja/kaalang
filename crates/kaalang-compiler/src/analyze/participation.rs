//! Checks that the questions and choices deciding whether a block executes
//! form a chain of nested selections rather than independent ones.
//!
//! Branch placement rejects independent selections even after disjoint partial
//! merges. This pass checks capture ancestry and the order of normal cycle exits.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Error, Result};

use super::{comparison, frame::Frames};
use crate::model::{BlockKind, Flow};

/// For every computational block, the questions and choices that decide it. A
/// question or choice decides a block when two executions select different
/// branches of it, agree at every other question or choice they both run, and
/// differ in whether the block participates. A question or choice on a path
/// that one execution cut short runs in only one of the two and takes no part
/// in the comparison. A repeating execution skips a block only when it neither
/// encloses nor passes that block before its tail. Each block compares the
/// selections its frame sees.
pub(super) fn deciders(flow: &Flow, frames: &Frames<'_>) -> Vec<BTreeSet<usize>> {
    let mut found = vec![BTreeSet::new(); flow.blocks.len() - 1];
    let mut by_frame = BTreeMap::<_, Vec<_>>::new();
    for block in 0..found.len() {
        by_frame.entry(frames.frame(block)).or_default().push(block);
    }
    for (frame, blocks) in by_frame {
        let executions = frames.view(frame).iter().collect::<Vec<_>>();
        let context = blocks
            .iter()
            .map(|&block| {
                executions
                    .iter()
                    .map(|execution| {
                        let runs = execution.participates(block);
                        (runs || frames.passes().reaches(execution, block)).then_some(runs)
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        comparison::compare(&executions, &context, |column, selector, first, second| {
            if context[column][first] != context[column][second] {
                found[blocks[column]].insert(selector);
            }
        });
    }
    found
}

/// The deciders of one block are pairwise dependent: one lies in the
/// continuation of a branch of the other, or after the cycle containing it exits.
/// Cycle exit order does not add captures or convergence groups. Two independent
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
                        && (super::cycle::closed_before(flow, first, second)
                            || super::cycle::closed_before(flow, second, first)))
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
            return Err(violation(flow, block));
        }
    }
    Ok(())
}

pub(crate) fn violation(flow: &Flow, block: usize) -> Error {
    // A boundary consumer is not authored: it carries its output's span.
    let message = if flow.blocks[block].transition_target.is_some() {
        let signal = flow.wire_name(&flow.blocks[block].inputs[0].ident);
        format!(
            "this kaalang stage transition exports `{signal}` on routes decided by two independent questions, choices, or cycles"
        )
    } else if flow.blocks[block].kind == BlockKind::Export {
        format!(
            "this kaalang cycle exports `{}` on routes decided by two independent questions, choices, or cycles",
            super::exported_name(flow, block)
        )
    } else {
        "this kaalang block must not be decided by two independent questions, choices, or cycles"
            .to_owned()
    };
    Error::new(flow.blocks[block].span, message)
}

#[cfg(test)]
mod tests {
    use super::deciders;
    use crate::{analyze::frame::Frames, build};

    #[test]
    fn participation_and_branch_order_match_the_pairwise_reference() {
        use kaalang_testing::corpus::{self, Suite};

        let corpus = corpus::corpus(Suite::All);
        corpus::assert_corpus_shape(&corpus, Suite::All);
        let generated = kaalang_testing::probes::accepted()
            .into_iter()
            .map(|(name, source, _)| (name, kaalang_testing::probes::flow(&source), false));
        for (name, function, _) in corpus.into_iter().chain(generated) {
            let parts = if let Some(part) = crate::analyze::walked(&function) {
                vec![part]
            } else {
                let analysis = crate::analyze(&function).expect("the staged flow analyzes");
                let mut parts = vec![(analysis.flow, analysis.executions.to_vec())];
                parts.extend(analysis.stages.into_iter().map(|stage| {
                    let local = *stage.analysis;
                    (local.flow, local.executions.to_vec())
                }));
                parts
            };
            for (flow, executions) in parts {
                let frames = Frames::of(&flow, &executions);
                for (block, actual) in deciders(&flow, &frames).into_iter().enumerate() {
                    let executions = frames
                        .at(block)
                        .iter()
                        .filter(|execution| {
                            execution.participates(block)
                                || frames.passes().reaches(execution, block)
                        })
                        .collect::<Vec<_>>();
                    let outcomes = executions
                        .iter()
                        .map(|execution| execution.participates(block))
                        .collect::<Vec<_>>();
                    let mut expected = std::collections::BTreeSet::new();
                    for (first, execution) in executions.iter().enumerate() {
                        for (second, other) in executions.iter().enumerate().skip(first + 1) {
                            if outcomes[first] != outcomes[second]
                                && let Some(selector) =
                                    crate::analyze::only_difference(execution, other)
                            {
                                expected.insert(selector);
                            }
                        }
                    }
                    assert_eq!(actual, expected, "{name}: block {block} deciders");
                    let mut ordered = (0..executions.len()).collect::<Vec<_>>();
                    ordered.sort_by_cached_key(|&index| {
                        executions[index]
                            .branches
                            .iter()
                            .filter(|selection| expected.contains(&selection.block))
                            .map(|selection| crate::BranchSelection {
                                block: selection.block,
                                branch: crate::analyze::frame::branch(selection.branch),
                            })
                            .collect::<Vec<_>>()
                    });
                    assert_eq!(
                        crate::analyze::branch_order(&executions, &outcomes),
                        ordered,
                        "{name}: block {block} branch order"
                    );
                }
            }
        }
    }

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
    /// the body's own selections stay inside it.
    #[test]
    fn a_cycle_with_several_outputs_decides_the_blocks_after_it() {
        let source = include_str!("../../../kaalang/tests/cycle/behavior/alternative_outputs.rs");
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
