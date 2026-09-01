//! Analyzes authored merge readiness and convergence.

use std::collections::HashSet;

use syn::{Error, Result};

use super::{Analysis, Exit, MergePlan, PathState, Producer, combine_states};
use crate::model::Block;

/// A merge is ready when any one of its alternative inputs is available.
pub(super) fn ready(block: &Block, state: &PathState) -> bool {
    block
        .inputs
        .iter()
        .any(|input| state.available.contains_key(&input.ident))
}

/// Validates branch convergence and computes the post-merge state.
pub(super) fn plan(analysis: &mut Analysis<'_>, exits: &[Exit]) -> Result<Option<MergePlan>> {
    let blocks = &analysis.flow.blocks;
    let arrivals = exits
        .iter()
        .filter_map(|exit| match exit {
            Exit::Merge {
                index,
                input,
                state,
            } => Some((*index, input, state)),
            Exit::Terminal(_) => None,
        })
        .collect::<Vec<_>>();
    let Some((index, _, _)) = arrivals.first().copied() else {
        return Ok(None);
    };
    if let Some((other, _, _)) = arrivals.iter().find(|(other, _, _)| *other != index) {
        return Err(Error::new(
            blocks[*other].span,
            "continuing branches must reach the same Contour merge",
        ));
    }

    let merge = &blocks[index];
    let mut arrived = HashSet::new();
    for (_, input, _) in &arrivals {
        if !arrived.insert((*input).clone()) {
            return Err(Error::new(
                input.span(),
                "two branches reach the same Contour merge input",
            ));
        }
    }
    let declared = merge
        .inputs
        .iter()
        .map(|input| input.ident.clone())
        .collect::<HashSet<_>>();
    if arrived != declared {
        return Err(Error::new(
            merge.span,
            "every Contour merge input must be reached by exactly one branch",
        ));
    }

    let states = arrivals
        .iter()
        .map(|(_, _, state)| *state)
        .collect::<Vec<_>>();
    let mut state = combine_states(&states);
    for exit in exits {
        state.executed.extend(exit.state().executed.iter().copied());
    }
    for input in &merge.inputs {
        state.available.remove(&input.ident);
    }
    state.produce(&merge.outputs[0], Producer::Block { index, output: 0 })?;
    state.executed.insert(index);
    state.last = Some(index);
    analysis.visited.insert(index);

    Ok(Some(MergePlan { index, state }))
}
