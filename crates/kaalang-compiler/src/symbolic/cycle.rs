//! The authored ordering of completing and repeating routes through a cycle.

use syn::Result;

use super::{Executions, condition::NEVER, frame::Frames};
use crate::Flow;

pub(super) fn output_order(
    executions: &mut Executions,
    flow: &Flow,
    frames: &Frames,
) -> Result<()> {
    let mut headers: Vec<_> = flow
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(header, block)| block.has_alternative_outputs().then_some(header))
        .collect();
    headers.sort_by_key(|&header| flow.blocks[header].cycle_end);
    for header in headers {
        let cycle = &flow.blocks[header];
        let repeat = executions.repeats(header);
        let repeat = executions.conditions.and(executions.domain, repeat);
        let mut outcomes = vec![repeat];
        for consumer in flow.exports(header) {
            outcomes.push(
                executions
                    .conditions
                    .and(executions.domain, executions.runs[consumer]),
            );
        }
        let context = outcomes
            .iter()
            .fold(NEVER, |sum, &when| executions.conditions.or(sum, when));
        let selectors =
            executions.outcome_selectors(&outcomes, frames.in_frame(Some(header)), context);
        let ordered = executions.ordered(&outcomes, &selectors);
        if let Some((before, after)) = ordered.gap_outputs {
            return Err(crate::analyze::cycle::repeat_between(cycle, before, after));
        }
        if let Some((first, later)) = ordered.descent {
            return Err(crate::analyze::cycle::out_of_order(cycle, first, later));
        }
    }
    Ok(())
}
