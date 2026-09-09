//! Resolves the implicit end block's `result` capture and rejects the blocks
//! an execution would still run after it has produced `result`.

use syn::Error;

use super::{Block, CaptureDependency, CaptureId, State, Walk};

/// Resolves the one wire end captures, recording the occurrence this execution
/// finishes with.
pub(super) fn arrive(walk: &mut Walk<'_>, state: &mut State) -> bool {
    let Some(&producer) = state.available.get(&walk.result) else {
        walk.incomplete.get_or_insert_with(|| {
            Error::new(
                walk.result.span(),
                "this kaalang execution does not produce the `result` wire",
            )
        });
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

/// `result` finishes an execution, so the block producing it is the last
/// participating one in source order.
pub(super) fn after_result(block: &Block) -> Error {
    Error::new(
        block.span,
        "this kaalang block runs after the `result` wire finishes its execution; declare it above the block producing `result`",
    )
}
