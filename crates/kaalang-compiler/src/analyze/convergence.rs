//! Derives convergence groups from dependency chains and validates the case
//! ranges of a choice's groups, independently of lowering.

use std::collections::{BTreeMap, BTreeSet};

use syn::Result;

use crate::model::{BlockKind, BranchSelection, ConvergenceGroup, Flow};

use super::frame::Frames;

/// Returns every convergence group in canonical order, or the earliest
/// crossing or separated choice group. Divergence after a convergence is the
/// wire merges' rule: a block gated by branch-local work cannot follow one.
pub(super) fn flow(
    flow: &Flow,
    frames: &Frames<'_>,
    precedence: &[Vec<BTreeSet<usize>>],
    ancestry: &[BTreeSet<BranchSelection>],
) -> Result<Vec<ConvergenceGroup>> {
    let mut recorded = Vec::new();
    for (brancher, declaration) in flow.blocks.iter().enumerate() {
        if declaration.branch_count() == 0 {
            continue;
        }
        let executions = frames.at(brancher);
        // A cycle's continuation starts after its body, which only makes the
        // selection.
        let past_body = |block: usize| declaration.loop_end.is_none_or(|end| block >= end);
        // Choice-like branchers compare their groups by route: the routes
        // each block continues, with the case each selected.
        let cases = matches!(declaration.kind, BlockKind::Choice | BlockKind::Loop);
        let mut branch_sets = vec![BTreeSet::new(); flow.blocks.len()];
        let route_of = cases.then(|| super::choice::routes(ancestry, executions, brancher));
        let mut routes = vec![BTreeMap::new(); flow.blocks.len()];
        let mut selected = Vec::new();
        for (index, (execution, preceding)) in executions.iter().zip(precedence).enumerate() {
            let Some(selection) = execution.branches.iter().find(|s| s.block == brancher) else {
                continue;
            };
            let route = route_of.as_ref().and_then(|route_of| route_of[index]);
            for &block in &execution.blocks {
                if preceding[block].contains(&brancher)
                    && past_body(block)
                    && frames.visible(frames.frame(block), brancher)
                {
                    branch_sets[block].insert(super::frame::branch(selection.branch));
                    if let Some((route, case)) = route {
                        routes[block].insert(route, case);
                    }
                }
            }
            if let Some(route) = route {
                selected.push((execution, route));
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
        if cases {
            // Keep until merge validation is proven to imply this check.
            // Ownership requires an `only_difference` pair, not just ancestry
            // (`question_after_a_partial_merge`). Other branchers filter contexts;
            // projection must still preserve adjacency and nesting. Unions alone
            // do not: {A, B} union {D} skips C. Generated searches are not a proof.
            //
            // Blocks continuing the same routes form one group. A repeat may
            // pass a block before reaching its cycle, so each block has its
            // own set of reaching routes.
            let mut continued = BTreeSet::new();
            for (block, (branches, routes)) in branch_sets.iter().zip(routes).enumerate() {
                if branches.len() < 2 {
                    continue;
                }
                let reaching = selected
                    .iter()
                    .filter(|(execution, _)| frames.passes().reaches(execution, block))
                    .map(|&(_, route)| route)
                    .collect::<BTreeMap<_, _>>();
                continued.insert((
                    branches.iter().copied().collect::<Vec<_>>(),
                    routes,
                    reaching,
                ));
            }
            let continued = continued
                .into_iter()
                .map(|(cases, routes, reaching)| super::choice::Group {
                    cases,
                    routes,
                    reaching,
                })
                .collect::<Vec<_>>();
            super::choice::validate_groups(declaration, &continued)?;
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
