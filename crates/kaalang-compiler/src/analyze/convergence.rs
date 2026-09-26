//! Derives convergence groups from dependency chains and validates the case
//! ranges of a choice's groups, independently of lowering.

use std::collections::{BTreeMap, BTreeSet};

use syn::Result;

use crate::model::{BlockKind, BranchSelection, ConvergenceGroup, Execution, Flow};

/// Returns every convergence group in canonical order, or the earliest
/// crossing or separated choice group. Divergence after a convergence is the
/// wire merges' rule: a block gated by branch-local work cannot follow one.
pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    precedence: &[Vec<BTreeSet<usize>>],
    ancestry: &[BTreeSet<BranchSelection>],
) -> Result<Vec<ConvergenceGroup>> {
    let mut recorded = Vec::new();
    for (brancher, declaration) in flow.blocks.iter().enumerate() {
        if !matches!(declaration.kind, BlockKind::Question | BlockKind::Choice) {
            continue;
        }
        let mut branch_sets = vec![BTreeSet::new(); flow.blocks.len()];
        // A choice's groups are compared by route: the routes each block
        // continues, with the case each selected.
        let route_of = (declaration.kind == BlockKind::Choice)
            .then(|| super::choice::routes(ancestry, executions, brancher));
        let mut routes = vec![BTreeMap::new(); flow.blocks.len()];
        let mut selected = Vec::new();
        for (index, (execution, preceding)) in executions.iter().zip(precedence).enumerate() {
            let Some(selection) = execution.branches.iter().find(|s| s.block == brancher) else {
                continue;
            };
            let route = route_of.as_ref().and_then(|route_of| route_of[index]);
            for &block in &execution.blocks {
                if preceding[block].contains(&brancher) {
                    branch_sets[block].insert(selection.branch);
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
        if declaration.kind == BlockKind::Choice {
            // Keep until merge validation is proven to imply this check.
            // Ownership requires an `only_difference` pair, not just ancestry
            // (`question_after_a_partial_merge`). Other branchers filter contexts;
            // projection must still preserve adjacency and nesting. Unions alone
            // do not: {A, B} union {D} skips C. Generated searches are not a proof.
            //
            // Blocks continuing the same routes form one group. A repeat of a
            // cycle not enclosing a block stops short of it, so the routes
            // reaching a block depend only on its level.
            let mut continued = BTreeSet::new();
            let mut reaching = BTreeMap::new();
            for (block, (branches, routes)) in branch_sets.iter().zip(routes).enumerate() {
                if branches.len() < 2 {
                    continue;
                }
                let reaching = reaching
                    .entry(flow.level(block))
                    .or_insert_with(|| {
                        selected
                            .iter()
                            .filter(|(execution, _)| flow.reaches(execution, block))
                            .map(|&(_, route)| route)
                            .collect::<BTreeMap<_, _>>()
                    })
                    .clone();
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
            super::choice::validate_groups(&declaration.outputs, &continued)?;
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
