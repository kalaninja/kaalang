//! Replays a lowering plan against every validated execution before using it.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    BlockKind, Branch, CaptureDependency, CaptureId, Execution, ExecutionPlan, Flow, Join,
    JoinTarget, ProducerId,
};

pub(super) fn plan(flow: &Flow, plan: &ExecutionPlan, executions: &[Execution]) -> bool {
    let mut bodies = vec![0; flow.blocks.len() - 1];
    count(plan, &mut bodies);
    bodies.iter().all(|&count| count == 1)
        && executions.iter().all(|execution| {
            let mut replay = Replay {
                flow,
                execution,
                available: flow
                    .flow_inputs
                    .iter()
                    .enumerate()
                    .map(|(index, name)| (name.clone(), ProducerId::FlowInput(index)))
                    .collect(),
                ran: BTreeSet::new(),
                dependencies: BTreeSet::new(),
            };
            matches!(replay.walk(plan), Some(Exit::End))
                && replay
                    .ran
                    .iter()
                    .copied()
                    .eq(execution.blocks.iter().copied())
                && replay
                    .dependencies
                    .iter()
                    .copied()
                    .eq(execution.dependencies.iter().copied())
        })
}

fn count(plan: &ExecutionPlan, bodies: &mut [usize]) {
    match plan {
        ExecutionPlan::Guarded { blocks, .. } => {
            for &block in blocks {
                bodies[block] += 1;
            }
        }
        ExecutionPlan::Action { index, next } => {
            bodies[*index] += 1;
            count(next, bodies);
        }
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => {
            bodies[*index] += 1;
            for branch in branches {
                count(&branch.plan, bodies);
            }
            if let Some(join) = join {
                count(&join.next, bodies);
            }
        }
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => {
            bodies[*index] += 1;
            for branch in branches {
                count(&branch.plan, bodies);
            }
            for join in joins {
                count(&join.next, bodies);
            }
        }
        ExecutionPlan::End { body, .. } => count(body, bodies),
        ExecutionPlan::EndArrival { .. } | ExecutionPlan::Yield { .. } => {}
    }
}

enum Exit {
    End,
    Yield(JoinTarget),
}

struct Replay<'a> {
    flow: &'a Flow,
    execution: &'a Execution,
    available: BTreeMap<Ident, ProducerId>,
    ran: BTreeSet<usize>,
    dependencies: BTreeSet<CaptureDependency>,
}

impl Replay<'_> {
    fn capture(&mut self, block: usize) -> Option<()> {
        for (input, declaration) in self.flow.blocks[block].inputs.iter().enumerate() {
            let producer = *self.available.get(&declaration.ident)?;
            self.dependencies.insert(CaptureDependency {
                producer,
                capture: CaptureId { block, input },
            });
            if !declaration.borrowed {
                self.available.remove(&declaration.ident);
            }
        }
        Some(())
    }

    fn enter(&mut self, block: usize, kind: BlockKind) -> Option<()> {
        if self.flow.blocks[block].kind != kind
            || !self.execution.participates(block)
            || !self.ran.insert(block)
        {
            return None;
        }
        self.capture(block)?;
        let selected = self
            .execution
            .branches
            .iter()
            .find(|selection| selection.block == block);
        for (output, name) in self.flow.blocks[block].outputs.iter().enumerate() {
            if selected.is_none_or(|selection| selection.branch == output) {
                self.available
                    .insert(name.clone(), ProducerId::BlockOutput { block, output });
            }
        }
        Some(())
    }

    fn walk(&mut self, plan: &ExecutionPlan) -> Option<Exit> {
        match plan {
            ExecutionPlan::Guarded { inputs, blocks } => {
                if !inputs
                    .iter()
                    .all(|input| self.available.contains_key(input))
                {
                    return None;
                }
                self.available.retain(|wire, _| inputs.contains(wire));
                for &block in blocks {
                    if self.flow.blocks[block]
                        .inputs
                        .iter()
                        .all(|input| self.available.contains_key(&input.ident))
                    {
                        self.enter(block, self.flow.blocks[block].kind)?;
                    }
                }
                self.capture(self.flow.blocks.len() - 1)?;
                Some(Exit::End)
            }
            ExecutionPlan::Action { index, next } => {
                self.enter(*index, BlockKind::Action)?;
                self.walk(next)
            }
            ExecutionPlan::Question {
                index,
                branches,
                join,
            } => {
                self.enter(*index, BlockKind::Question)?;
                self.branch(*index, branches, join.as_slice())
            }
            ExecutionPlan::Choice {
                index,
                branches,
                joins,
            } => {
                self.enter(*index, BlockKind::Choice)?;
                self.branch(*index, branches, joins)
            }
            ExecutionPlan::End { body, .. } => self.walk(body),
            ExecutionPlan::EndArrival { inputs } => {
                let end = self.flow.blocks.len() - 1;
                if !inputs.iter().eq(self.flow.blocks[end]
                    .inputs
                    .iter()
                    .map(|input| &input.ident))
                {
                    return None;
                }
                self.capture(end)?;
                Some(Exit::End)
            }
            ExecutionPlan::Yield { wires, join } => wires
                .iter()
                .all(|wire| self.available.contains_key(wire))
                .then_some(Exit::Yield(*join)),
        }
    }

    fn branch(&mut self, block: usize, branches: &[Branch], joins: &[Join]) -> Option<Exit> {
        let selected = self
            .execution
            .branches
            .iter()
            .find(|selection| selection.block == block)?
            .branch;
        let outside = self
            .available
            .iter()
            .filter(|(_, producer)| {
                !matches!(producer, ProducerId::BlockOutput { block: source, .. } if *source == block)
            })
            .map(|(wire, _)| wire.clone())
            .collect::<BTreeSet<_>>();
        match self.walk(&branches.get(selected)?.plan)? {
            Exit::Yield(target) if target.block == block => {
                let join = joins.get(target.join)?;
                if !join.branches.contains(&selected) {
                    return None;
                }
                self.available
                    .retain(|wire, _| outside.contains(wire) || join.wires.contains(wire));
                self.walk(&join.next)
            }
            exit => Some(exit),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn rejects_a_plan_that_skips_an_independent_effect() {
        let model = crate::build(&parse_quote! {
            fn effects() {
                #[action("First")]
                || -> () {};
                #[action("Second")]
                || -> () {};
                #[end]
                || {};
            }
        })
        .expect("the independent effects are valid");
        assert!(plan(&model.flow, &model.execution_plan, &model.executions));
        let incomplete = ExecutionPlan::Action {
            index: 0,
            next: Box::new(ExecutionPlan::EndArrival { inputs: Vec::new() }),
        };
        assert!(!plan(&model.flow, &incomplete, &model.executions));
    }

    #[test]
    fn rejects_a_join_that_drops_a_needed_binding() {
        let mut model = crate::build(&parse_quote! {
            fn choose(flag: bool) -> u8 {
                #[question("Choose")]
                |flag| -> (yes, no) { flag };
                #[action("Yes value")]
                |yes| -> value { 1u8 };
                #[action("No value")]
                |no| -> value { 2u8 };
                #[action("Use the value")]
                |value| -> result { value };
                #[end]
                |result| {};
            }
        })
        .expect("the alternative producers are valid");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Question {
            join: Some(join), ..
        } = body.as_mut()
        else {
            unreachable!()
        };
        join.wires.clear();
        assert!(!plan(&model.flow, &model.execution_plan, &model.executions));
    }
}
