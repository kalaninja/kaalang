//! Analyzes the ordered branches of a choice.

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{Analysis, PathState, Walked};
use crate::model::Plan;

/// Walks every case branch and records their shared continuation.
pub(super) fn walk(analysis: &mut Analysis<'_>, index: usize, state: PathState) -> Result<Walked> {
    let next = analysis.enter(index, state);
    let outputs = &analysis.flow.blocks[index].outputs;
    let mut walked = Vec::with_capacity(outputs.len());
    for output in outputs {
        let mut branch_state = next.clone();
        branch_state.available.insert(output.clone());
        walked.push(analysis.walk(branch_state)?);
    }
    adjacent_branches(&walked, outputs)?;
    let (branches, merge, exit) = analysis.branches(walked)?;

    Ok(Walked {
        plan: Plan::Choice {
            index,
            branches,
            merge,
        },
        exit,
    })
}

/// Rejects a case that ends the flow between two cases that continue, which no
/// skewer order can draw: the later continuing branch reaches its merge by
/// crossing the ending branch. A question cannot reach this, because a merge it
/// feeds is reached by both of its branches or by neither.
fn adjacent_branches(walked: &[Walked], outputs: &[Ident]) -> Result<()> {
    let continuing = |path: &Walked| !path.exit.terminates();
    let Some(first) = walked.iter().position(continuing) else {
        return Ok(());
    };
    let last = walked
        .iter()
        .rposition(continuing)
        .expect("a continuing branch was just found");
    let Some(offset) = walked[first..last]
        .iter()
        .position(|path| !continuing(path))
    else {
        return Ok(());
    };

    Err(Error::new(
        outputs[first + offset].span(),
        "a Contour case that ends the flow must not separate cases that continue to a merge",
    ))
}
