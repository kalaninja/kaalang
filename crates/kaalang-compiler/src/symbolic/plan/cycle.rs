//! Lowers and universally replays one finite cycle frame.

use std::collections::BTreeSet;

use super::{Builder, Lowered, Replay, Scope};
use crate::{
    BlockKind, Branch, ExecutionOutcome, ExecutionPlan, Join,
    symbolic::condition::{Condition, NEVER},
};

pub(super) fn lower(
    builder: &mut Builder<'_>,
    header: usize,
    context: Condition,
    done: &BTreeSet<usize>,
    forbidden: &BTreeSet<usize>,
    scopes: &[Scope],
) -> Lowered {
    let end = builder.flow.blocks[header]
        .cycle_end
        .expect("a cycle owns a body");
    let continuation: BTreeSet<_> = (end..builder.flow.blocks.len() - 1).collect();
    let mut inside_forbidden = forbidden.clone();
    inside_forbidden.extend(&continuation);
    let mut inside_scopes = scopes.to_vec();
    inside_scopes.push(Scope {
        block: header,
        groups: vec![continuation],
        from: 0,
    });
    let body = builder.lower(context, done, &inside_forbidden, &inside_scopes);
    let completing = builder.flow.exports(header).fold(NEVER, |sum, consumer| {
        builder
            .executions
            .conditions
            .or(sum, builder.executions.runs[consumer])
    });
    let completing = builder.executions.conditions.and(context, completing);
    let mut emitted = body.emitted;
    emitted.insert(header);
    let mut finished = done.clone();
    finished.extend(header + 1..end);
    let (branches, joins, yielding) = if completing == NEVER {
        (Vec::new(), Vec::new(), NEVER)
    } else if builder.flow.blocks[header].branch_count() > 0 {
        let continued = builder.branch(header, completing, &finished, forbidden, scopes);
        emitted.extend(continued.emitted);
        let ExecutionPlan::Choice {
            branches, joins, ..
        } = continued.plan
        else {
            unreachable!("cycle alternatives use choice-like continuations")
        };
        (branches, joins, continued.yielding)
    } else {
        let next = builder.lower(completing, &finished, forbidden, scopes);
        emitted.extend(next.emitted);
        (vec![Box::new(next.plan)], Vec::new(), next.yielding)
    };
    Lowered {
        plan: ExecutionPlan::Cycle {
            index: header,
            body: Box::new(body.plan),
            branches,
            joins,
        },
        emitted,
        yielding,
    }
}

pub(super) fn replay(
    replay: &mut Replay<'_>,
    header: usize,
    body: &ExecutionPlan,
    branches: &[Branch],
    joins: &[Join],
    context: Condition,
) {
    replay.enter(header, context, BlockKind::Cycle);
    let outside = replay.available.clone();
    replay.cycle_indices.push(header);
    replay.visit(body, context);
    replay.cycle_indices.pop();
    let completed = replay.exports.remove(&header).unwrap_or(NEVER);
    let ended = replay.returned.values().fold(completed, |sum, &when| {
        replay.executions.conditions.or(sum, when)
    });
    replay.requires(context, ended);
    for producer in replay
        .available
        .keys()
        .chain(outside.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let current = replay.available.get(&producer).copied().unwrap_or(NEVER);
        let retained = replay.executions.conditions.minus(current, completed);
        let outer = outside.get(&producer).copied().unwrap_or(NEVER);
        let restored = replay.executions.conditions.and(outer, completed);
        replay.available.insert(
            producer,
            replay.executions.conditions.or(retained, restored),
        );
    }
    replay.produce(header, completed);
    if replay.flow.blocks[header].branch_count() > 0 {
        replay.branches(header, branches, joins, completed);
    } else if completed != NEVER {
        if let [next] = branches {
            replay.visit(next, completed);
        } else {
            replay.valid = false;
        }
    }
}

pub(super) fn export(replay: &mut Replay<'_>, index: usize, target: usize, context: Condition) {
    replay.enter(index, context, BlockKind::Export);
    if replay.flow.blocks[index].export_target != Some(target)
        || replay.cycle_indices.last() != Some(&target)
    {
        replay.valid = false;
        return;
    }
    if replay.flow.blocks[target].branch_count() > 0 {
        let selected = replay
            .executions
            .selected(target, replay.flow.exported_output(index));
        replay.requires(context, selected);
    }
    let entry = replay.exports.entry(target).or_insert(NEVER);
    *entry = replay.executions.conditions.or(*entry, context);
}

pub(super) fn repeat(replay: &mut Replay<'_>, index: usize, context: Condition) {
    replay.enter(index, context, BlockKind::Continue);
    let Some(target) = replay.flow.blocks[index].parent else {
        replay.valid = false;
        return;
    };
    if replay.cycle_indices.last() != Some(&target) {
        replay.valid = false;
    }
    let entry = replay
        .returned
        .entry(ExecutionOutcome::Repeat {
            cycle_index: target,
        })
        .or_insert(NEVER);
    *entry = replay.executions.conditions.or(*entry, context);
}
