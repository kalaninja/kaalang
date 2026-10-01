//! Conditional exports close one body route.

use super::{Condition, State, Walk};
use syn::Error;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &mut State, mut runs: Condition) {
    let header = walk.flow.blocks[block]
        .export_target
        .expect("an export has an owning cycle");
    let output = walk.flow.exported_output(block);
    for (other, consumer) in walk
        .flow
        .exports(header)
        .enumerate()
        .filter(|&(other, _)| other != output)
    {
        let present = walk.present(
            &state.available,
            &walk.flow.blocks[consumer].inputs[0].ident,
        );
        let both = walk.executions.conditions.and(runs, present);
        if walk.has(both) {
            let (first, second) = (output.min(other), output.max(other));
            let cycle = &walk.flow.blocks[header];
            walk.report((block, 0), Error::new(cycle.output_binding(second).ident.span(), format!("a route through this kaalang cycle produces both `{}` and `{}`; a completed cycle exports exactly one of its outputs", cycle.output_binding(first).ident, cycle.output_binding(second).ident)));
            state.live = walk.executions.conditions.minus(state.live, both);
            runs = walk.executions.conditions.minus(runs, both);
        }
    }
    walk.exports[block] = runs;
    state.live = walk.executions.conditions.minus(state.live, runs);
}
