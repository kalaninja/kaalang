//! Conditional repeats close one finite iteration.

use syn::Error;

use super::{Condition, State, Walk};
use crate::ExecutionOutcome;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, mut runs: Condition) {
    let header = walk.flow.blocks[block]
        .parent
        .expect("a continue has an owning cycle");
    for (output, consumer) in walk.flow.exports(header).enumerate() {
        let present = walk.present(
            &state.available,
            &walk.flow.blocks[consumer].inputs[0].ident,
        );
        let both = walk.executions.conditions.and(runs, present);
        if walk.has(both) {
            walk.report((block, 0), Error::new(walk.flow.blocks[block].span, format!("a route through this kaalang cycle produces its output `{}` and then repeats; a route either completes with one output or continues", walk.flow.blocks[header].output_binding(output).ident)));
            state.live = walk.executions.conditions.minus(state.live, both);
            runs = walk.executions.conditions.minus(runs, both);
        }
    }
    walk.finish(
        state,
        ExecutionOutcome::Repeat {
            cycle_index: header,
        },
        runs,
    );
}
