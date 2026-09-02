//! Validates one path's arrival at the authored End block.

use syn::{Error, Result};

use super::{
    Analysis, PathState,
    frontier::{WorkKind, WorkPlan},
};

pub(super) fn arrive(
    analysis: &mut Analysis<'_>,
    index: usize,
    state: PathState,
) -> Result<WorkPlan> {
    let block = &analysis.flow.blocks[index];
    let inputs = block
        .inputs
        .iter()
        .map(|input| {
            state
                .available
                .contains_key(&input.ident)
                .then(|| input.ident.clone())
                .ok_or_else(|| {
                    Error::new(
                        input.ident.span(),
                        "this kaalang path cannot supply every End input",
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let state = analysis.enter(index, state);
    if let Some(wire) = analysis
        .flow
        .sources
        .iter()
        .chain(analysis.flow.blocks.iter().flat_map(|block| &block.outputs))
        .filter(|wire| !wire.to_string().starts_with('_'))
        .find_map(|wire| state.unconsumed.get(wire))
    {
        return Err(Error::new(
            wire.span(),
            "every non-ignored kaalang wire must have a consumer on every path",
        ));
    }

    Ok(WorkPlan {
        kind: WorkKind::EndArrival { inputs },
        exit: Some(state),
    })
}
