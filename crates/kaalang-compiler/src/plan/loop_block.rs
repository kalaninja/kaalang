//! Emits one cycle body and the continuations its declared outputs reach.

use std::collections::BTreeSet;

use super::{Builder, Lowered, Scope, Unstructured};
use crate::model::{Branch, Execution, ExecutionPlan};

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
    let completing = executions
        .iter()
        .copied()
        .filter(|execution| builder.flow.completes_loop(execution, block))
        .collect::<Vec<_>>();
    let mut emitted = body.emitted;
    emitted.insert(block);
    let mut finished = done.clone();
    finished.extend(block + 1..end);
    let (branches, joins, yielding) = if completing.is_empty() {
        (Vec::new(), Vec::new(), Vec::new())
    } else if builder.flow.blocks[block].branch_count() > 0 {
        // Several outputs continue like the cases of a choice.
        let (branches, joins, yielding, continued) =
            builder.continuations(block, &completing, &finished, forbidden, scopes)?;
        emitted.extend(continued);
        (branches, joins, yielding)
    } else {
        let next = builder.lower(&completing, &finished, forbidden, scopes)?;
        emitted.extend(next.emitted);
        let branch = Box::new(next.plan);
        (vec![branch], Vec::new(), next.yielding)
    };
    Ok(Lowered {
        plan: ExecutionPlan::Loop {
            index: block,
            body: Box::new(body.plan),
            branches,
            joins,
        },
        emitted,
        yielding,
    })
}

pub(super) fn replay(
    replay: &mut super::verify::Replay<'_>,
    index: usize,
    body: &ExecutionPlan,
    branches: &[Branch],
    joins: &[crate::model::Join],
) -> Option<super::verify::Exit> {
    use super::verify::Exit;
    replay.enter(index, crate::model::BlockKind::Loop)?;
    let outside = replay.available.clone();
    match replay.iteration(index, body)? {
        Exit::Export(target) if target == index => {
            replay.available = outside;
            replay.produce(index);
            if replay.flow.blocks[index].branch_count() > 0 {
                replay.branch(index, branches, joins)
            } else {
                replay.walk(branches.first()?)
            }
        }
        exit @ (Exit::Return(_) | Exit::Repeat(_) | Exit::Export(_)) => Some(exit),
        Exit::Yield(_) => None,
    }
}
