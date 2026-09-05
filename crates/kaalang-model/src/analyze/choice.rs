//! Explores a choice's outputs and validates their capture coverage.

use std::collections::BTreeSet;

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{State, Walk};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    walk.branch(block, state);
}

/// Every ordinary choice output needs a consumer in some execution.
pub(super) fn uncaptured(output: &Ident) -> Error {
    Error::new(
        output.span(),
        "every kaalang choice output must have a consumer",
    )
}

/// A choice's dependency-derived groups are disjoint and adjacent.
pub(super) fn validate_groups(
    outputs: &[Ident],
    groups: &[(Vec<usize>, BTreeSet<usize>)],
) -> Result<()> {
    let mut offending = None::<(usize, &str)>;
    let mut report = |case: usize, message: &'static str| {
        if offending.is_none_or(|(earliest, _)| case < earliest) {
            offending = Some((case, message));
        }
    };
    for (position, (group, _)) in groups.iter().enumerate() {
        if let (Some(&first), Some(&last)) = (group.first(), group.last())
            && let Some(gap) = (first..=last).find(|case| !group.contains(case))
        {
            report(
                gap,
                "branches in a kaalang choice convergence group must be adjacent",
            );
        }
        for (other, _) in &groups[position + 1..] {
            if let Some(&shared) = group.iter().find(|case| other.contains(case)) {
                report(
                    shared,
                    "a kaalang case must not enter two convergence groups",
                );
            }
        }
    }
    match offending {
        Some((case, message)) => Err(Error::new(outputs[case].span(), message)),
        None => Ok(()),
    }
}
