//! Conditional completion restores the cycle's outer data scope.

use syn::Error;

use super::{Condition, NEVER, State, Walk};

pub(super) fn visit(
    walk: &mut Walk<'_>,
    header: usize,
    mut outside: State,
    runs: Condition,
) -> State {
    let end = walk.flow.blocks[header]
        .loop_end
        .expect("a cycle owns a body");
    let body = walk.sequence(
        header + 1,
        end,
        State {
            live: runs,
            available: outside.available.clone(),
        },
    );
    if walk.has(body.live) {
        let message = if walk.flow.blocks[header].outputs.is_empty() {
            "a route through this kaalang cycle reaches the end of its body; an outputless cycle repeats it with `continue`"
        } else {
            "a route through this kaalang cycle reaches the end of its body without a declared output; produce one of its outputs or repeat it with `continue`"
        };
        walk.report(
            (end - 1, usize::MAX),
            Error::new(walk.flow.blocks[header].span, message),
        );
    }
    let mut completed = NEVER;
    for (output, consumer) in walk.flow.exports(header).enumerate() {
        let when = walk.exports[consumer];
        let provided = walk.produce(&mut outside.available, header, output, when);
        completed = walk.executions.conditions.or(completed, provided);
    }
    walk.constrain(header, completed);
    if walk.flow.blocks[header].branch_count() > 0 {
        for (output, consumer) in walk.flow.exports(header).enumerate() {
            let selected = walk.executions.selected(header, output);
            let agrees = walk
                .executions
                .conditions
                .xor(selected, walk.exports[consumer]);
            let allowed = walk.executions.conditions.not(agrees);
            walk.executions.domain = walk
                .executions
                .conditions
                .and(walk.executions.domain, allowed);
        }
    }
    let skipped = walk.executions.conditions.minus(outside.live, runs);
    outside.live = walk.executions.conditions.or(skipped, completed);
    outside
}
