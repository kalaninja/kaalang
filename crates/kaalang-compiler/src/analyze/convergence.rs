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
            // Keep until merge validation is proven to imply this check.
            // Ownership requires an `only_difference` pair, not just ancestry
            // (`question_after_a_partial_merge`). Other branchers filter contexts;
            // projection must still preserve adjacency and nesting. Unions alone
            // do not: {A, B} union {D} skips C. Generated searches are not a proof.
            super::choice::validate_groups(&declaration.outputs, &groups)?;
        }
        for (branches, shared) in groups {
            recorded.push(ConvergenceGroup {
                branching_block: brancher,
                branches,
                continuation: shared.into_iter().collect(),
            });
        }
    }
    Ok(recorded)
}
