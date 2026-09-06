//! Resolves the implicit end block's `result` capture and rejects the blocks
//! that could still be ready once the flow can finish.

use syn::Error;

use super::{Block, CaptureDependency, CaptureId, State, Walk};

/// Resolves the one wire end captures, recording the occurrence this execution
/// finishes with.
pub(super) fn arrive(walk: &mut Walk<'_>, state: &mut State) -> bool {
    let Some(&producer) = state.available.get(&walk.result) else {
        walk.report(
            (walk.end, 0),
            Error::new(
                walk.result.span(),
                "this kaalang execution does not produce the `result` wire",
            ),
        );
        return false;
    };
    state.dependencies.insert(CaptureDependency {
        producer,
        capture: CaptureId {
            block: walk.end,
            input: 0,
        },
    });
    true
}

/// A block ready alongside end never runs before the flow finishes: its work
/// reaches nothing the end block waits for.
pub(super) fn still_ready(block: &Block) -> Error {
    Error::new(
        block.span,
        "this kaalang block can still be ready once the `result` wire is available; its work must reach `result`",
    )
}
