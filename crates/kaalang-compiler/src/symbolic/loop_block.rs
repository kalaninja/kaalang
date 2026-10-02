//! The authored ordering of completing and repeating routes through a cycle.

use syn::{Error, Result};

use super::{Executions, condition::NEVER, frame::Frames};
use crate::{ExecutionOutcome, Flow};

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
    headers.sort_by_key(|&header| flow.blocks[header].loop_end);
    for header in headers {
        let cycle = &flow.blocks[header];
        let repeat = executions
            .outcomes
            .get(&ExecutionOutcome::Repeat { loop_index: header })
            .copied()
            .unwrap_or(NEVER);
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
            return Err(Error::new(
                cycle.span,
                format!(
                    "a route repeating this kaalang cycle lies between routes exporting `{}` and `{}`; move the repeating routes to one edge of the body",
                    cycle.output_binding(before).ident,
                    cycle.output_binding(after).ident
                ),
            ));
        }
        if let Some((first, later)) = ordered.descent {
            return Err(Error::new(
                cycle.output_binding(first).ident.span(),
                format!(
                    "the routes exporting `{}` leave this kaalang cycle to the left of those exporting `{}`; declare its outputs in the order their routes leave it",
                    cycle.output_binding(first).ident,
                    cycle.output_binding(later).ident
                ),
            ));
        }
    }
    Ok(())
}
