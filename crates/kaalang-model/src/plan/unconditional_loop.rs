//! Emits one unconditional loop body and no after-loop continuation.

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
    let mut emitted = body.emitted;
    emitted.insert(block);
    Ok(Lowered {
        plan: ExecutionPlan::Loop {
            index: block,
            body: Box::new(body.plan),
        },
        emitted,
        yielding: body.yielding,
    })
}

pub(super) fn replay(
    replay: &mut super::verify::Replay<'_>,
    index: usize,
    body: &ExecutionPlan,
) -> Option<super::verify::Exit> {
    use super::verify::Exit;
    replay.enter(index, crate::model::BlockKind::Loop)?;
    match replay.iteration(index, body)? {
        exit @ (Exit::End | Exit::Repeat(_)) => Some(exit),
        Exit::Yield(_) => None,
    }
}
