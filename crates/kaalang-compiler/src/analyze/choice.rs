//! Explores a choice's branches and validates its convergence groups.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{State, Walk};
use crate::model::{BranchSelection, Execution};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    walk.branch(block, state);
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
            let case = execution.selected(choice)?;
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
    /// The routes that reach the level where this group joins, taken or not,
    /// each with its case. A repeat of a cycle not enclosing the group stops at
    /// that cycle's tail and never reaches it.
    pub(super) reaching: BTreeMap<usize, usize>,
}

/// Choice groups occupy adjacent cases and may nest, but may not cross. Both
/// are decided by route, not by case: a later selection inside one case may
/// send its routes to different groups, and only a route that reaches a group
/// can separate it or set it apart from another.
pub(super) fn validate_groups(outputs: &[Ident], groups: &[Group]) -> Result<()> {
    let mut offending = None::<(usize, &str)>;
    let mut report = |case: usize, message: &'static str| {
        if offending.is_none_or(|(earliest, _)| case < earliest) {
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
                "branches in a kaalang choice convergence group must be adjacent",
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
                    "kaalang choice convergence groups must be disjoint or nested",
                );
            }
        }
    }
    match offending {
        Some((case, message)) => Err(Error::new(outputs[case].span(), message)),
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
