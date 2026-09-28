//! Resolves stage interfaces and checks each local sequence and their finite graph.

use std::collections::{BTreeSet, VecDeque};

use proc_macro2::Ident;
use syn::{Error, FnArg, ItemFn, Pat, Result};

use crate::model::{
    Analysis, Execution, ExecutionOutcome, ExecutionPlan, Flow, ProducerId, StageAnalysis,
};
use crate::parse::ParsedStaged;
use crate::{analyze_local, plan, resolve, scope};

fn available(flow: &Flow, route: &Execution, name: &Ident) -> bool {
    flow.flow_inputs.contains(name)
        || flow.blocks.iter().enumerate().any(|(block, declaration)| {
            declaration.parent.is_none()
                && declaration
                    .outputs
                    .iter()
                    .enumerate()
                    .any(|(output, wire)| {
                        wire == name
                            && flow
                                .produces(route, crate::ProducerId::BlockOutput { block, output })
                    })
        })
}

/// Wires retained by preparation's outer scope, and the block where initial
/// transitions begin. A transition bypassing a join keeps that join's locals
/// inside preparation; only the common prefix can surround the dispatcher.
pub(crate) fn preparation_scope(flow: &Flow, mut plan: &ExecutionPlan) -> (BTreeSet<Ident>, usize) {
    let mut wires = flow.flow_inputs.iter().cloned().collect::<BTreeSet<_>>();
    loop {
        match plan {
            ExecutionPlan::End { body, .. } => plan = body,
            ExecutionPlan::Action { index, next } | ExecutionPlan::Call { index, next } => {
                wires.extend(flow.blocks[*index].outputs.iter().cloned());
                plan = next;
            }
            ExecutionPlan::Loop {
                index, branches, ..
            } if flow.blocks[*index].branch_count() == 0 => {
                wires.extend(flow.blocks[*index].outputs.iter().cloned());
                let Some(branch) = branches.first() else {
                    return (wires, *index);
                };
                plan = &branch.plan;
            }
            ExecutionPlan::Question {
                index,
                branches,
                joins,
            }
            | ExecutionPlan::Choice {
                index,
                branches,
                joins,
            }
            | ExecutionPlan::Loop {
                index,
                branches,
                joins,
                ..
            } => {
                let Some((join, earlier)) = joins.split_last() else {
                    return (wires, *index);
                };
                let mut before_join = Vec::new();
                for branch in branches {
                    plan::serial_order(&branch.plan, &mut before_join);
                }
                for join in earlier {
                    plan::serial_order(&join.next, &mut before_join);
                }
                if before_join
                    .iter()
                    .any(|&block| flow.blocks[block].transition_target.is_some())
                {
                    return (wires, *index);
                }
                wires.extend(join.wires.iter().cloned());
                plan = &join.next;
            }
            ExecutionPlan::Return { index }
            | ExecutionPlan::Continue { index }
            | ExecutionPlan::Export { index, .. } => return (wires, *index),
            ExecutionPlan::Yield { .. } => {
                unreachable!("preparation's outer scope does not yield to another join")
            }
        }
    }
}

fn targets(analysis: &Analysis) -> impl Iterator<Item = usize> + '_ {
    analysis.executions.iter().filter_map(|route| {
        let ExecutionOutcome::Return { block_index } = route.outcome else {
            return None;
        };
        analysis.flow.blocks[block_index].transition_target
    })
}

/// Stages in the order preparation's transitions first visit them; a stage
/// missing from it is unreachable. Rust infers an entry's type from the first
/// constructor it checks, and a stage's first constructor is in preparation or
/// an earlier visited stage, so the dispatcher emits its arms in this order.
pub(crate) fn visit_order(preparation: &Analysis, stages: &[StageAnalysis]) -> Vec<usize> {
    let mut visited = vec![false; stages.len()];
    let mut order = Vec::new();
    let mut queue = targets(preparation).collect::<VecDeque<_>>();
    while let Some(stage) = queue.pop_front() {
        if std::mem::replace(&mut visited[stage], true) {
            continue;
        }
        order.push(stage);
        queue.extend(targets(&stages[stage].analysis));
    }
    order
}

fn mutable_outer(function: &ItemFn, preparation: &Flow, name: &Ident) -> bool {
    match preparation.producer(name) {
        Some(ProducerId::BlockOutput { block, output }) => {
            preparation.blocks[block].output_binding(output).mutability.is_some()
        }
        Some(ProducerId::FlowInput(index)) => match function.sig.inputs.iter().filter(|argument| {
            !matches!(argument, FnArg::Typed(argument) if matches!(argument.pat.as_ref(), Pat::Wild(_)))
        }).nth(index).expect("each flow input comes from a named parameter") {
            FnArg::Typed(argument) => matches!(argument.pat.as_ref(), Pat::Ident(binding) if binding.mutability.is_some()),
            FnArg::Receiver(receiver) => crate::parse::receiver_capture(receiver).2,
        },
        None => false,
    }
}

#[allow(clippy::too_many_lines)] // Coordinates local analyses with the shared preparation boundary.
pub(crate) fn analyze(function: &ItemFn, mut parsed: ParsedStaged) -> Result<Analysis> {
    scope::resolve(&mut parsed.preparation)?;
    resolve::flow(&parsed.preparation)?;
    let preliminary = analyze_local(function, parsed.preparation, false)?;
    let (outer, _) = preparation_scope(&preliminary.flow, &preliminary.execution_plan);
    let mut preparation = preliminary.flow;
    let completed = preliminary
        .executions
        .iter()
        .filter(|route| matches!(route.outcome, ExecutionOutcome::Return { .. }))
        .collect::<Vec<_>>();
    let candidates = preparation
        .flow_inputs
        .iter()
        .cloned()
        .chain(
            preparation
                .blocks
                .iter()
                .filter(|block| block.parent.is_none())
                .flat_map(|block| block.outputs.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let entry_names = parsed
        .stages
        .iter()
        .map(|stage| stage.entry.clone())
        .collect::<BTreeSet<_>>();
    let provided = candidates
        .iter()
        .filter(|name| {
            !entry_names.contains(*name)
                && completed
                    .iter()
                    .all(|route| available(&preparation, route, name))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let common = provided
        .intersection(&outer)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut stages = Vec::new();
    let mut stage_spans = Vec::new();
    let mut used_common = BTreeSet::new();
    for mut stage in parsed.stages.drain(..) {
        stage
            .flow
            .flow_inputs
            .retain(|name| name == &stage.entry || common.contains(name));
        scope::resolve(&mut stage.flow)?;
        for input in stage.flow.blocks.iter().flat_map(|block| &block.inputs) {
            if candidates.contains(&input.ident)
                && !entry_names.contains(&input.ident)
                && !common.contains(&input.ident)
                && stage.flow.producer(&input.ident).is_none()
            {
                return Err(Error::new(
                    input.alias.span(),
                    if provided.contains(&input.ident) {
                        "a branch-local preparation wire must be merged before a kaalang stage can capture it"
                    } else {
                        "a common outer wire used by a kaalang stage must be available on every preparation route"
                    },
                ));
            }
            if input.borrowed
                && input.mutable
                && common.contains(&input.ident)
                && !mutable_outer(function, &preparation, &input.ident)
            {
                return Err(Error::new(
                    input.alias.span(),
                    "a mutable capture requires the outer wire to be declared with `mut`",
                ));
            }
        }
        resolve::flow(&stage.flow)?;
        used_common.extend(
            stage
                .flow
                .blocks
                .iter()
                .flat_map(|block| &block.inputs)
                .map(|input| &input.ident)
                .filter(|name| common.contains(*name))
                .cloned(),
        );
        stage_spans.push(stage.span);
        let analysis = analyze_local(function, stage.flow, true)?;
        stages.push(StageAnalysis {
            description: stage.description,
            entry: stage.entry,
            entry_alias: stage.entry_alias,
            analysis: Box::new(analysis),
        });
    }

    for block in preparation
        .blocks
        .iter_mut()
        .filter(|block| block.transition_target.is_some())
    {
        for wire in &used_common {
            block.inputs.push(crate::Input {
                borrowed: false,
                mutable: false,
                ident: wire.clone(),
                alias: wire.clone(),
                derived: true,
            });
        }
    }
    resolve::flow(&preparation)?;
    let mut preparation = analyze_local(function, preparation, true)?;
    let order = visit_order(&preparation, &stages);
    if let Some(index) = (0..stages.len()).find(|stage| !order.contains(stage)) {
        return Err(Error::new(
            stage_spans[index],
            "this kaalang stage is unreachable from preparation",
        ));
    }
    preparation.stages = stages;
    preparation.common_wires = used_common.into_iter().collect();
    Ok(preparation)
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    #[test]
    fn resolves_a_self_transition_and_terminal_stage() {
        let function: ItemFn = parse_quote! {
            fn count_to(limit: usize) -> usize {
                #[action("Initialize the counter.")]
                let mut counter = || 0usize;

                #[action("Begin counting.")]
                let count = || ();

                #[stage("Count to the limit.")]
                let (count, finish) = |count| {
                    #[question("Done?")]
                    let (finish, again) = |counter, limit| counter >= limit;

                    #[action("Increment and repeat.")]
                    let count = |again, &mut counter| { *counter += 1; };
                };

                #[stage("Return the count.")]
                |finish| {
                    |counter| return counter;
                };
            }
        };
        let analysis = crate::analyze(&function).expect("staged flow analyzes");
        assert_eq!(analysis.stages.len(), 2);
        assert_eq!(analysis.stages[0].entry, "count");
        assert_eq!(analysis.stages[1].entry, "finish");
        crate::expand(function).expect("the staged flow lowers");
    }

    #[test]
    fn terminal_stage_must_be_declared_last() {
        let function = crate::tests::fixture(
            include_str!("../../kaalang/tests/stage/behavior/is_even.rs"),
            "is_even",
        );
        for position in 1..=3 {
            let mut function = function.clone();
            let terminal = function.block.stmts.remove(3);
            function.block.stmts.insert(position, terminal);
            let result = crate::analyze(&function);
            if position < 3 {
                assert_eq!(
                    result.err().unwrap().to_string(),
                    "a terminal kaalang stage must be declared last"
                );
            } else {
                result.expect("a final terminal stage is valid");
            }
        }
    }

    #[test]
    fn accepts_a_stage_that_diverges_in_a_nested_cycle() {
        let function: ItemFn = parse_quote! {
            fn diverge() {
                #[action("Enter the only stage.")]
                let go = || ();

                #[stage("Repeat forever.")]
                |go| {
                    #[cycle("Stay in this stage.")]
                    {
                        continue;
                    };
                };
            }
        };
        let model = crate::build(&function).expect("the divergent stage has a diagram");
        assert_eq!(model.stages.len(), 1);
        assert!(
            model.stages[0]
                .topology
                .nodes
                .iter()
                .all(|node| node.kind != crate::topology::NodeKind::End)
        );
        crate::expand(function).expect("the dispatcher can diverge");
    }
}
