//! Connects executions that disagree at exactly one shared selector.
//! See RFC 0007 §5.1 for the comparison contexts.

use super::only_difference;
use crate::model::Execution;

/// Compares every pair of executions. `None` excludes an execution from that
/// context. The callback receives its context index, the differing selector
/// and the two execution indices. Production analysis enumerates at most
/// [`crate::symbolic::ENUMERATED_HISTORY_LIMIT`] executions, so this needs
/// nothing built up front.
pub(super) fn compare<T>(
    executions: &[&Execution],
    context: &[Vec<Option<T>>],
    mut compare: impl FnMut(usize, usize, usize, usize),
) {
    for (position, first) in executions.iter().enumerate() {
        for (offset, second) in executions[position + 1..].iter().enumerate() {
            let Some(selector) = only_difference(first, second) else {
                continue;
            };
            let other = position + 1 + offset;
            for (column, context) in context.iter().enumerate() {
                let (Some(_), Some(_)) = (&context[position], &context[other]) else {
                    continue;
                };
                compare(column, selector, position, other);
            }
        }
    }
}
