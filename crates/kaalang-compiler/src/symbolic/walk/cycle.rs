//! Conditional completion restores the cycle's outer data scope.

use super::{Condition, NEVER, State, Walk};

pub(super) fn visit(
    walk: &mut Walk<'_>,
    header: usize,
    mut outside: State,
    runs: Condition,
) -> State {
    let end = walk.flow.blocks[header]
        .cycle_end
        .expect("a cycle owns a body");
    let body = walk.sequence(
        header + 1,
        end,
        State {
            live: runs,
            available: outside.available.clone(),
            resume: Vec::new(),
        },
    );
    if walk.has(body.live) {
        let error = crate::analyze::cycle::open_body(walk.flow, header);
        walk.report((end - 1, usize::MAX), error);
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
            let c = &mut walk.executions.conditions;
            let differs = c.xor(selected, walk.exports[consumer]);
            walk.executions.domain = c.minus(walk.executions.domain, differs);
        }
    }
    let skipped = walk.executions.conditions.minus(outside.live, runs);
    outside.live = walk.executions.conditions.or(skipped, completed);
    outside
}
