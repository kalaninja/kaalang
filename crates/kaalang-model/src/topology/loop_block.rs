//! Loop statements use structural junctions, not computational nodes.

use std::collections::BTreeMap;

use super::{Analyzed, Connection, Source, Topology, destination, represented};

pub(super) fn project(
    block: usize,
    has_inputs: bool,
    loops: &[super::Loop],
    count: &mut usize,
) -> Option<usize> {
    loops
        .iter()
        .find(|entry| entry.header == block)
        .map_or_else(
            || {
                has_inputs.then(|| {
                    let index = *count;
                    *count += 1;
                    index
                })
            },
            |entry| Some(entry.entry),
        )
}

/// Records the exited region's precedence. Projection carries it through the
/// continuation to its merge, iteration tail, or end boundary.
pub(super) fn order_exits(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
    topology: &mut Topology,
) {
    for execution in model.executions {
        for (position, &block) in execution.blocks.iter().enumerate() {
            let Some(target) = model.flow.blocks[block].break_target else {
                continue;
            };
            let Some(&next) = execution.blocks[position + 1..]
                .iter()
                .find(|&&next| represented(model, structural, next))
            else {
                continue;
            };
            let end = model.flow.blocks[target]
                .loop_end
                .expect("a loop owns a body");
            for loop_ in topology
                .loops
                .iter()
                .filter(|loop_| (target..end).contains(&loop_.header))
            {
                topology.order.push(Connection {
                    source: Source::Junction(loop_.tail),
                    destination: destination(structural, next),
                });
            }
        }
    }
}

/// Follow the side occupied by the repeating routes of the first selection.
/// Without a common rightmost branch, the return starts on the left contour.
pub(super) fn prefer_left(model: &Analyzed<'_>, header: usize) -> bool {
    let end = model.flow.blocks[header]
        .loop_end
        .expect("a loop owns a body");
    let Some(first) = (header + 1..end).find(|&index| model.flow.blocks[index].branch_count() > 0)
    else {
        return true;
    };
    let last = model.flow.blocks[first].branch_count() - 1;
    model
        .executions
        .iter()
        .filter(|execution| execution.repeats.contains(&header))
        .any(|execution| execution.selected(first) != Some(last))
}
