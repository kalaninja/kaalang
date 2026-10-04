//! Explores a choice's branches and validates its convergence groups.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Error, Result};

use super::{State, Walk};
use crate::model::{Block, BlockKind, BranchSelection, Execution};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    // A for cycle out of items passes its body by, up to its export.
    let exhausted = walk.flow.blocks[block]
        .parent
        .filter(|_| walk.flow.takes_next_item(block))
        .map(|header| walk.flow.exports(header).start);
    walk.branch(block, state, |case| match exhausted {
        Some(export) if case == 1 => export,
        _ => block + 1,
    });
}

/// Each execution's route through `choice`, with its case, or `None` for an
/// execution that selects none of its cases. A route is the case followed by
/// the selections nested inside it; selections below the point where the cases
/// converge belong to every case alike and tell no group apart. Executions
/// taking the same route share one id.
pub(super) fn routes(
    ancestry: &[BTreeSet<BranchSelection>],
    executions: &[Execution],
    choice: usize,
) -> Vec<Option<(usize, usize)>> {
    let mut ids = BTreeMap::<Vec<BranchSelection>, usize>::new();
    executions
        .iter()
        .map(|execution| {
            let case = super::frame::branch(execution.selected(choice)?);
            let own = BranchSelection {
                block: choice,
                branch: case,
            };
            let route = execution
                .branches
                .iter()
                .filter(|selection| {
                    selection.block == choice || ancestry[selection.block].contains(&own)
                })
                .copied()
                .collect();
            let next = ids.len();
            Some((*ids.entry(route).or_insert(next), case))
        })
        .collect()
}

/// One convergence group of a choice, seen per route.
pub(super) struct Group {
    /// The cases whose routes take this group.
    pub(super) cases: Vec<usize>,
    /// The routes this group takes, by id, each with its case.
    pub(super) routes: BTreeMap<usize, usize>,
    /// The routes that reach the position where this group joins, taken or not,
    /// each with its case. A repeating route may pass the join before its tail.
    pub(super) reaching: BTreeMap<usize, usize>,
}

/// Choice groups occupy adjacent cases and may nest, but may not cross. Both
/// are decided by route, not by case: a later selection inside one case may
/// send its routes to different groups, and only a route that reaches a group
/// can separate it or set it apart from another. A cycle's declared outputs
/// group the same way.
pub(super) fn validate_groups(owner: &Block, groups: &[Group]) -> Result<()> {
    let (noun, members) = match owner.kind {
        BlockKind::Cycle => ("cycle", "outputs"),
        _ => ("choice", "branches"),
    };
    let mut offending = None::<(usize, String)>;
    let mut report = |case: usize, message: String| {
        if offending
            .as_ref()
            .is_none_or(|(earliest, _)| case < *earliest)
        {
            offending = Some((case, message));
        }
    };
    for (position, group) in groups.iter().enumerate() {
        let reaching = group.reaching.values().collect::<BTreeSet<_>>();
        if let (Some(&first), Some(&last)) = (group.cases.first(), group.cases.last())
            && let Some(gap) =
                (first..=last).find(|case| !group.cases.contains(case) && reaching.contains(case))
        {
            report(
                gap,
                format!("{members} in a kaalang {noun} convergence group must be adjacent"),
            );
        }
        for other in &groups[position + 1..] {
            let shared = group
                .routes
                .iter()
                .find_map(|(route, &case)| other.routes.contains_key(route).then_some(case));
            if let Some(shared) = shared
                && apart(group, other)
                && apart(other, group)
            {
                report(
                    shared,
                    format!("kaalang {noun} convergence groups must be disjoint or nested"),
                );
            }
        }
    }
    match offending {
        Some((case, message)) => Err(Error::new(owner.outputs[case].span(), message)),
        None => Ok(()),
    }
}

/// Whether `group` takes a route that `other` reaches but does not take. Only
/// a route the other group could have taken sets this one apart.
fn apart(group: &Group, other: &Group) -> bool {
    group
        .routes
        .keys()
        .any(|route| !other.routes.contains_key(route) && other.reaching.contains_key(route))
}
