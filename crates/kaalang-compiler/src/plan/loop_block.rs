//! Emits one cycle body and the continuation reached by its breaks.

use std::collections::BTreeSet;

use super::{Builder, Lowered, Scope, Unstructured};
use crate::model::{Execution, ExecutionPlan};

pub(super) fn lower<'e>(
    builder: &mut Builder<'_>,
    block: usize,
    executions: &[&'e Execution],
    done: &BTreeSet<usize>,
    forbidden: &BTreeSet<usize>,
    scopes: &[Scope],
) -> Result<Lowered<'e>, Unstructured> {
    let end = builder.flow.blocks[block]
        .loop_end
        .expect("a loop owns a body");
    let continuation = (end..builder.end).collect::<BTreeSet<_>>();
    let mut inside_forbidden = forbidden.clone();
    inside_forbidden.extend(&continuation);
    let mut inside_scopes = scopes.to_vec();
    inside_scopes.push(Scope {
        block,
        groups: vec![continuation],
        from: 0,
    });
    let body = builder.lower(executions, done, &inside_forbidden, &inside_scopes)?;
    let normal = executions
        .iter()
        .copied()
        .filter(|execution| {
            execution
                .blocks
                .iter()
                .any(|&index| builder.flow.blocks[index].break_target == Some(block))
        })
        .collect::<Vec<_>>();
    let mut emitted = body.emitted;
    emitted.insert(block);
    let mut yielding = Vec::new();
    let next = if normal.is_empty() {
        None
    } else {
        let mut finished = done.clone();
        finished.extend(block + 1..end);
        let next = builder.lower(&normal, &finished, forbidden, scopes)?;
        emitted.extend(next.emitted);
        yielding = next.yielding;
        Some(Box::new(next.plan))
    };
    Ok(Lowered {
        plan: ExecutionPlan::Loop {
            index: block,
            body: Box::new(body.plan),
            next,
        },
        emitted,
        yielding,
    })
}

pub(super) fn replay(
    replay: &mut super::verify::Replay<'_>,
    index: usize,
    body: &ExecutionPlan,
    next: Option<&ExecutionPlan>,
) -> Option<super::verify::Exit> {
    use super::verify::Exit;
    replay.enter(index, crate::model::BlockKind::Loop)?;
    let outside = std::mem::replace(&mut replay.available, replay.flow.cycle_bindings(index));
    match replay.iteration(index, body)? {
        Exit::Break(target) if target == index => {
            replay.available = outside;
            replay.produce(index);
            replay.walk(next?)
        }
        exit @ (Exit::Return(_) | Exit::Repeat(_) | Exit::Break(_)) => Some(exit),
        Exit::Yield(_) => None,
    }
}
