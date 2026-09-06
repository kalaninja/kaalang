//! Derives convergence groups from dependency chains and validates the case
//! ranges of a choice's groups, independently of lowering.

use std::collections::{BTreeMap, BTreeSet};

use syn::Result;

use crate::model::{BlockKind, ConvergenceGroup, Execution, Flow};

/// Returns every convergence group in canonical order, or the earliest
/// crossing or separated choice group. Divergence after a convergence is the
/// wire merges' rule: a block gated by branch-local work cannot follow one.
pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    precedence: &[Vec<BTreeSet<usize>>],
) -> Result<Vec<ConvergenceGroup>> {
    let mut recorded = Vec::new();
    for (brancher, declaration) in flow.blocks.iter().enumerate() {
        if !matches!(declaration.kind, BlockKind::Question | BlockKind::Choice) {
            continue;
        }
        let mut branch_sets = vec![BTreeSet::new(); flow.blocks.len()];
        for (execution, preceding) in executions.iter().zip(precedence) {
            let Some(selection) = execution.branches.iter().find(|s| s.block == brancher) else {
                continue;
            };
            for &block in &execution.blocks {
                if preceding[block].contains(&brancher) {
                    branch_sets[block].insert(selection.branch);
                }
            }
        }
        let mut groups = BTreeMap::<Vec<usize>, BTreeSet<usize>>::new();
        for (block, branches) in branch_sets.iter().enumerate() {
            if branches.len() >= 2 {
                groups
                    .entry(branches.iter().copied().collect())
                    .or_default()
                    .insert(block);
            }
        }
        let groups = groups.into_iter().collect::<Vec<_>>();
        if declaration.kind == BlockKind::Choice {
            // Keep this check: redundancy after merge validation is unproven.
            // Merge groups of each owner are adjacent and nested or disjoint.
            // With other branch outcomes fixed, changing a merge owner's
            // outcome cannot change downstream participation when both
            // executions produce the wire: that would put the downstream
            // block in `before` and create a cycle. This suffices for flows
            // with only one branching block.
            //
            // The gap is inheritance across other branchers' outcomes. An
            // owner needs an `only_difference` pair selecting different
            // producers; ancestry alone does not imply ownership (see
            // `question_after_a_partial_merge`). Other branchers can filter
            // producing contexts, so we cannot simply treat a block's case
            // set as an intersection of validated merge groups. We still
            // need to prove that projecting the remaining executions onto
            // this choice preserves adjacency and nesting, even when brancher
            // dependencies exist only in some executions. Unions alone do
            // not suffice: {A, B} union {D} skips C.
            // Generated-flow searches found no rejection unique to this
            // check, but do not establish that missing inheritance argument.
            super::choice::validate_groups(&declaration.outputs, &groups)?;
        }
        for (branches, shared) in groups {
            let entries = entries(&shared, precedence);
            recorded.push(ConvergenceGroup {
                branching_block: brancher,
                branches,
                continuation: shared.into_iter().collect(),
                entries,
            });
        }
    }
    Ok(recorded)
}

/// The continuation blocks that no other continuation block precedes in any
/// execution. `preceding` is transitive, so a chain through any intermediate
/// block counts.
fn entries(shared: &BTreeSet<usize>, precedence: &[Vec<BTreeSet<usize>>]) -> Vec<usize> {
    shared
        .iter()
        .copied()
        .filter(|&entry| {
            precedence
                .iter()
                .all(|preceding| preceding[entry].is_disjoint(shared))
        })
        .collect()
}
