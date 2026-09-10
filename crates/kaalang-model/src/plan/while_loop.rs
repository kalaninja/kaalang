//! Emits the body once and keeps the continuation outside the loop.

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
    let declaration = &builder.flow.blocks[block];
    let end = declaration.loop_end.expect("a while owns a body");
    let yes = declaration.yes_branch();
    let iteration = executions
        .iter()
        .copied()
        .filter(|execution| execution.selected(block) == Some(yes))
        .collect::<Vec<_>>();
    let continuation = (end..builder.end).collect::<BTreeSet<_>>();
    let mut inside_forbidden = forbidden.clone();
    inside_forbidden.extend(&continuation);
    let mut inside_scopes = scopes.to_vec();
    inside_scopes.push(Scope {
        block,
        groups: vec![continuation],
        from: 0,
    });
    let body = builder.lower(&iteration, done, &inside_forbidden, &inside_scopes)?;
    let normal = executions
        .iter()
        .copied()
        .filter(|execution| {
            execution.selected(block) != Some(yes) || execution.repeats.contains(&block)
        })
        .collect::<Vec<_>>();
    let mut finished = done.clone();
    finished.extend(block + 1..end);
    let next = builder.lower(&normal, &finished, forbidden, scopes)?;
    let mut emitted = body.emitted;
    emitted.extend(next.emitted);
    emitted.insert(block);
    Ok(Lowered {
        plan: ExecutionPlan::While {
            index: block,
            body: Box::new(body.plan),
            next: Box::new(next.plan),
        },
        emitted,
        yielding: next.yielding,
    })
}

/// Replays the structural one-iteration summary; the runtime loop is not unrolled.
pub(super) fn replay(
    replay: &mut super::verify::Replay<'_>,
    index: usize,
    body: &ExecutionPlan,
    next: &ExecutionPlan,
) -> Option<super::verify::Exit> {
    use super::verify::Exit;
    replay.enter(index, crate::model::BlockKind::While)?;
    if replay.execution.selected(index)? == replay.flow.blocks[index].yes_branch() {
        let outside = replay.available.clone();
        match replay.iteration(index, body)? {
            Exit::Repeat(target) if target == index => {
                replay.available = outside;
            }
            Exit::End => return Some(Exit::End),
            exit @ Exit::Repeat(_) => return Some(exit),
            _ => return None,
        }
    }
    replay.walk(next)
}
