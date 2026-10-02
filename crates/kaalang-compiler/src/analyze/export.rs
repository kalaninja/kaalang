//! Exports one declared output at the end of its cycle's body.

use syn::Error;

use super::{State, Walk};
use crate::model::BranchSelection;

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let target = walk.flow.blocks[block]
        .export_target
        .expect("a boundary consumer has a cycle");
    debug_assert_eq!(
        state.cycles.last_key_value().map(|(&index, _)| index),
        Some(target)
    );
    let output = walk.flow.exported_output(block);
    let cycle = &walk.flow.blocks[target];
    if let Some(other) =
        super::produced_outputs(walk.flow, &state, target).find(|&other| other != output)
    {
        let (first, second) = (output.min(other), output.max(other));
        walk.report(
            (block, 0),
            Error::new(
                cycle.output_binding(second).ident.span(),
                format!(
                    "a route through this kaalang cycle produces both `{}` and `{}`; a completed cycle exports exactly one of its outputs",
                    cycle.output_binding(first).ident,
                    cycle.output_binding(second).ident
                ),
            ),
        );
        return;
    }
    state.available = state
        .cycles
        .remove(&target)
        .expect("the target cycle is active");
    if cycle.branch_count() > 0 {
        state.branches.insert(BranchSelection {
            block: target,
            branch: output,
        });
    }
    if walk.produce(&mut state, target, output) {
        walk.visit(cycle.cycle_end.expect("a cycle owns a body"), state);
    }
}
