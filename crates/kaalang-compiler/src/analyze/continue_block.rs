//! Ends one iteration at the transfer that repeats its cycle.

use syn::Error;

use super::{State, Walk};
use crate::model::ExecutionOutcome;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: State) {
    let header = walk.flow.blocks[block]
        .parent
        .expect("a continue belongs to a cycle");
    debug_assert_eq!(
        state.cycles.last_key_value().map(|(&index, _)| index),
        Some(header)
    );
    // A route either completes with an output or repeats.
    if let Some(output) = super::produced_outputs(walk.flow, &state, header).next() {
        walk.report(
            (block, 0),
            Error::new(
                walk.flow.blocks[block].span,
                format!(
                    "a route through this kaalang cycle produces its output `{}` and then repeats; a route either completes with one output or continues",
                    walk.flow.blocks[header].output_binding(output).ident
                ),
            ),
        );
        return;
    }
    walk.record(
        state,
        ExecutionOutcome::Repeat {
            cycle_index: header,
        },
    );
}
