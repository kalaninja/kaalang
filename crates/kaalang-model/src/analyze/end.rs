//! Resolves the authored end block's captures after computational blocks finish.

use syn::Error;

use super::{CaptureDependency, CaptureId, State, Walk};

pub(super) fn arrive(walk: &mut Walk<'_>, state: &mut State) -> bool {
    for (index, input) in walk.flow.blocks[walk.end].inputs.iter().enumerate() {
        let Some(&producer) = state.available.get(&input.ident) else {
            walk.report(
                (walk.end, index),
                Error::new(
                    input.ident.span(),
                    "this kaalang execution cannot provide every flow output",
                ),
            );
            return false;
        };
        state.dependencies.insert(CaptureDependency {
            producer,
            capture: CaptureId {
                block: walk.end,
                input: index,
            },
        });
    }
    true
}
