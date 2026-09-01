//! Analyzes merge readiness, branch arrivals, and convergence.

use std::collections::HashSet;

use syn::{Error, Result};

use super::{Analysis, Exit, MergePlan, PathState, Walked, combine_states};
use crate::model::{Block, Plan};

/// A merge is ready when any one of its alternative inputs is available.
pub(super) fn ready(block: &Block, state: &PathState) -> bool {
    block
        .inputs
        .iter()
        .any(|input| state.available.contains(&input.ident))
}

/// Records which one of a merge's inputs the current branch reaches.
pub(super) fn walk(analysis: &Analysis<'_>, index: usize, state: PathState) -> Result<Walked> {
    let block = &analysis.flow.blocks[index];
    let available = block
        .inputs
        .iter()
        .filter(|input| state.available.contains(&input.ident))
        .collect::<Vec<_>>();
    debug_assert!(!available.is_empty(), "a ready merge has an input");
    if available.len() != 1 {
        return Err(Error::new(
            available[1].ident.span(),
            "exactly one Contour merge input must be available on each path",
        ));
    }

    let input = available[0].ident.clone();
    let mut state = state;
    state.available.remove(&input);
    Ok(Walked {
        plan: Plan::Arrival {
            input: input.clone(),
            merge: index,
        },
        exit: Exit::Merge {
            index,
            input,
            state,
        },
    })
}

/// Validates branch convergence and computes the post-merge state.
pub(super) fn plan(analysis: &mut Analysis<'_>, paths: &[Walked]) -> Result<Option<MergePlan>> {
    let blocks = &analysis.flow.blocks;
    let arrivals = paths
        .iter()
        .filter_map(|path| match &path.exit {
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
    for path in paths {
        state
            .executed
            .extend(path.exit.state().executed.iter().copied());
    }
    for input in &merge.inputs {
        state.available.remove(&input.ident);
    }
    state.produce(&merge.outputs[0])?;
    state.executed.insert(index);
    state.last = Some(index);
    analysis.visited.insert(index);

    Ok(Some(MergePlan { index, state }))
}
