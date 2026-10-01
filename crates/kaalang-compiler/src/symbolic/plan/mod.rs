//! Lowers execution sets without enumerating their members.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use super::{
    Executions,
    condition::{Condition, NEVER},
};
mod loop_block;

use crate::{BlockKind, Branch, ExecutionPlan, Flow, Join, JoinTarget, ProducerId, WireMerge};

struct Builder<'a> {
    executions: &'a mut Executions,
    flow: &'a Flow,
    merges: &'a [WireMerge],
    classes: Vec<BTreeSet<ProducerId>>,
}

struct Lowered {
    plan: ExecutionPlan,
    yielding: Condition,
    emitted: BTreeSet<usize>,
}

#[derive(Clone)]
struct Scope {
    block: usize,
    groups: Vec<BTreeSet<usize>>,
    from: usize,
}

type Group = (Vec<usize>, BTreeSet<usize>);

impl Executions {
    pub(crate) fn plan(&mut self, flow: &Flow, merges: &[WireMerge]) -> ExecutionPlan {
        let domain = self.domain;
        let mut builder = Builder {
            executions: self,
            flow,
            merges,
            classes: Vec::new(),
        };
        let lowered = builder.lower(domain, &BTreeSet::new(), &BTreeSet::new(), &[]);
        assert_eq!(
            lowered.yielding, NEVER,
            "every execution has a final outcome"
        );
        let gates = merges
            .iter()
            .filter(|merge| {
                !builder.classes.iter().any(|class| {
                    merge
                        .producers
                        .iter()
                        .all(|producer| class.contains(producer))
                })
            })
            .map(|merge| merge.wire.clone())
            .collect();
        let plan = ExecutionPlan::End {
            index: flow.blocks.len() - 1,
            body: Box::new(lowered.plan),
            gates,
        };
        assert!(
            self.verify(flow, &plan, merges),
            "the symbolic plan preserves every structural execution"
        );
        plan
    }
}

impl Builder<'_> {
    fn intersects(&mut self, context: Condition, when: Condition) -> bool {
        self.executions.conditions.and(context, when) != NEVER
    }

    fn covers(&mut self, context: Condition, when: Condition) -> bool {
        self.executions.conditions.minus(context, when) == NEVER
    }

    fn settled(&mut self, context: Condition, block: usize, done: &BTreeSet<usize>) -> bool {
        for (&dependency, &when) in &self.executions.dependencies.clone() {
            if dependency.capture.block == block
                && let ProducerId::BlockOutput { block, .. } = dependency.producer
                && !done.contains(&block)
                && self.intersects(context, when)
            {
                return false;
            }
        }
        for merge in self
            .merges
            .iter()
            .filter(|merge| merge.after.contains(&block))
        {
            for &before in &merge.before {
                if !done.contains(&before) && self.intersects(context, self.executions.runs[before])
                {
                    return false;
                }
            }
            for &producer in &merge.producers {
                if let ProducerId::BlockOutput { block, .. } = producer
                    && !done.contains(&block)
                    && self.intersects(context, self.executions.produced(producer))
                {
                    return false;
                }
            }
        }
        true
    }

    fn lower(
        &mut self,
        context: Condition,
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Lowered {
        assert_ne!(context, NEVER, "every branch has an execution");
        let block = (0..self.flow.blocks.len() - 1)
            .filter(|block| !done.contains(block) && !forbidden.contains(block))
            .find(|&block| {
                self.covers(context, self.executions.runs[block])
                    && self.settled(context, block, done)
            });
        let Some(block) = block else {
            return self.leaf(context, done, forbidden, scopes);
        };
        let mut next_done = done.clone();
        next_done.insert(block);
        match self.flow.blocks[block].kind {
            BlockKind::Action | BlockKind::Call => {
                let mut next = self.lower(context, &next_done, forbidden, scopes);
                next.emitted.insert(block);
                next.plan = if self.flow.blocks[block].kind == BlockKind::Call {
                    ExecutionPlan::Call {
                        index: block,
                        next: Box::new(next.plan),
                    }
                } else {
                    ExecutionPlan::Action {
                        index: block,
                        next: Box::new(next.plan),
                    }
                };
                next
            }
            BlockKind::Return => Lowered {
                plan: ExecutionPlan::Return { index: block },
                yielding: NEVER,
                emitted: BTreeSet::from([block]),
            },
            BlockKind::Question | BlockKind::Choice => {
                self.branch(block, context, &next_done, forbidden, scopes)
            }
            BlockKind::Loop => {
                loop_block::lower(self, block, context, &next_done, forbidden, scopes)
            }
            BlockKind::Export => Lowered {
                plan: ExecutionPlan::Export {
                    index: block,
                    target: self.flow.blocks[block]
                        .export_target
                        .expect("an export has a cycle"),
                },
                yielding: NEVER,
                emitted: BTreeSet::from([block]),
            },
            BlockKind::Continue => Lowered {
                plan: ExecutionPlan::Continue { index: block },
                yielding: NEVER,
                emitted: BTreeSet::from([block]),
            },
            BlockKind::End => unreachable!("the end is outside the authored sequence"),
        }
    }

    fn leaf(
        &mut self,
        context: Condition,
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Lowered {
        let waiting: BTreeSet<_> = forbidden
            .iter()
            .copied()
            .filter(|&block| {
                !done.contains(&block) && self.intersects(context, self.executions.runs[block])
            })
            .collect();
        assert!(!waiting.is_empty(), "a validated execution reaches a join");
        assert!(
            !(0..self.flow.blocks.len() - 1).any(|block| !done.contains(&block)
                && !forbidden.contains(&block)
                && self.intersects(context, self.executions.runs[block])),
            "all pending computation belongs to a join"
        );
        let waiting_when = waiting.iter().fold(NEVER, |sum, &block| {
            self.executions
                .conditions
                .or(sum, self.executions.runs[block])
        });
        assert!(
            self.covers(context, waiting_when),
            "every execution reaches its join"
        );
        let join = scopes
            .iter()
            .rev()
            .find_map(|scope| {
                scope
                    .groups
                    .iter()
                    .enumerate()
                    .skip(scope.from)
                    .find(|(_, group)| !group.is_disjoint(&waiting))
                    .map(|(join, _)| JoinTarget {
                        block: scope.block,
                        join,
                    })
            })
            .expect("a shared block belongs to an enclosing join");
        let assignment = self
            .executions
            .conditions
            .witness(context)
            .expect("the execution set is nonempty");
        let mut available: BTreeMap<_, _> = self
            .flow
            .flow_inputs
            .iter()
            .map(|name| (name.clone(), name.clone()))
            .collect();
        for &block in done {
            for (output, name) in self.flow.blocks[block].outputs.iter().enumerate() {
                let when = self
                    .executions
                    .produced(ProducerId::BlockOutput { block, output });
                if self.executions.conditions.matches(when, &assignment) {
                    available.insert(name.clone(), name.clone());
                }
            }
        }
        Lowered {
            plan: ExecutionPlan::Yield {
                wires: available.into_values().collect(),
                join,
            },
            yielding: context,
            emitted: BTreeSet::new(),
        }
    }

    fn groups(
        &mut self,
        selections: &[Condition],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
    ) -> Vec<Group> {
        let mut groups = BTreeMap::<Vec<usize>, Vec<(Condition, BTreeSet<usize>)>>::new();
        for block in (0..self.flow.blocks.len() - 1)
            .filter(|block| !done.contains(block) && !forbidden.contains(block))
        {
            let members: Vec<_> = selections
                .iter()
                .enumerate()
                .filter_map(|(branch, &context)| {
                    self.intersects(context, self.executions.runs[block])
                        .then_some(branch)
                })
                .collect();
            if members.len() < 2 {
                continue;
            }
            let all = selections.iter().fold(NEVER, |sum, &context| {
                self.executions.conditions.or(sum, context)
            });
            let context = self
                .executions
                .conditions
                .and(all, self.executions.runs[block]);
            let groups = groups.entry(members).or_default();
            if let Some((scope, blocks)) = groups.last_mut()
                && self.covers(context, *scope)
            {
                blocks.insert(block);
            } else {
                groups.push((context, BTreeSet::from([block])));
            }
        }
        let mut groups: Vec<_> = groups
            .into_iter()
            .flat_map(|(branches, groups)| {
                groups
                    .into_iter()
                    .map(move |(_, blocks)| (branches.clone(), blocks))
            })
            .collect();
        groups.sort_by_key(|(branches, _)| branches.len());
        groups
    }

    #[allow(clippy::too_many_lines)] // Keeps branch bodies and their successive joins together.
    fn branch(
        &mut self,
        block: usize,
        context: Condition,
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Lowered {
        let selections: Vec<_> = (0..self.flow.blocks[block].outputs.len())
            .map(|branch| {
                let selected = self.executions.selected(block, branch);
                self.executions.conditions.and(context, selected)
            })
            .collect();
        let groups = self.groups(&selections, done, forbidden);
        let shared: BTreeSet<_> = groups
            .iter()
            .flat_map(|(_, blocks)| blocks.iter().copied())
            .collect();
        let inner_forbidden = forbidden.union(&shared).copied().collect();
        let mut inner_scopes = scopes.to_vec();
        inner_scopes.push(Scope {
            block,
            groups: groups.iter().map(|(_, blocks)| blocks.clone()).collect(),
            from: 0,
        });
        let mut branches: Vec<_> = selections
            .iter()
            .map(|&selection| self.lower(selection, done, &inner_forbidden, &inner_scopes))
            .collect();
        let mut emitted: BTreeSet<_> = branches
            .iter()
            .flat_map(|branch| branch.emitted.iter().copied())
            .collect();
        let mut joined = done.clone();
        joined.extend(&emitted);
        let mut joins: Vec<Join> = Vec::new();
        let mut yielding = NEVER;
        for (join, (members, blocks)) in groups.iter().enumerate() {
            let candidates = members.iter().fold(NEVER, |sum, &branch| {
                self.executions
                    .conditions
                    .or(sum, branches[branch].yielding)
            });
            let runs = blocks.iter().fold(NEVER, |sum, &block| {
                self.executions
                    .conditions
                    .or(sum, self.executions.runs[block])
            });
            let context = self.executions.conditions.and(candidates, runs);
            let wires = self.join_wires(context, &joined, done);
            let target = JoinTarget { block, join };
            for branch in &mut branches {
                crate::plan::fill_yields(&mut branch.plan, &wires, target);
            }
            for earlier in &mut joins {
                crate::plan::fill_yields(&mut earlier.next, &wires, target);
            }
            let later: BTreeSet<_> = groups[join + 1..]
                .iter()
                .flat_map(|(_, blocks)| blocks.iter().copied())
                .collect();
            let inner_forbidden = forbidden.union(&later).copied().collect();
            let mut inner_scopes = scopes.to_vec();
            inner_scopes.push(Scope {
                block,
                groups: groups.iter().map(|(_, blocks)| blocks.clone()).collect(),
                from: join + 1,
            });
            let next = self.lower(context, &joined, &inner_forbidden, &inner_scopes);
            emitted.extend(&next.emitted);
            joined.extend(&next.emitted);
            let stays = later.iter().fold(NEVER, |sum, &block| {
                self.executions
                    .conditions
                    .or(sum, self.executions.runs[block])
            });
            let leaves = self.executions.conditions.minus(next.yielding, stays);
            yielding = self.executions.conditions.or(yielding, leaves);
            joins.push(Join {
                branches: members.clone(),
                wires,
                next: Box::new(next.plan),
            });
        }
        let runs_shared = shared.iter().fold(NEVER, |sum, &block| {
            self.executions
                .conditions
                .or(sum, self.executions.runs[block])
        });
        for branch in &branches {
            let leaves = self
                .executions
                .conditions
                .minus(branch.yielding, runs_shared);
            yielding = self.executions.conditions.or(yielding, leaves);
        }
        emitted.insert(block);
        let branches = branches
            .into_iter()
            .map(|branch| Branch {
                plan: Box::new(branch.plan),
            })
            .collect();
        let plan = if self.flow.blocks[block].kind == BlockKind::Question {
            ExecutionPlan::Question {
                index: block,
                branches,
                joins,
            }
        } else {
            ExecutionPlan::Choice {
                index: block,
                branches,
                joins,
            }
        };
        Lowered {
            plan,
            yielding,
            emitted,
        }
    }

    fn join_wires(
        &mut self,
        context: Condition,
        done: &BTreeSet<usize>,
        outside: &BTreeSet<usize>,
    ) -> Vec<Ident> {
        let mut available = BTreeMap::<Ident, Vec<(ProducerId, Condition)>>::new();
        for (index, name) in self.flow.flow_inputs.iter().enumerate() {
            available
                .entry(name.clone())
                .or_default()
                .push((ProducerId::FlowInput(index), self.executions.domain));
        }
        for &block in done {
            for (output, name) in self.flow.blocks[block].outputs.iter().enumerate() {
                let producer = ProducerId::BlockOutput { block, output };
                let when = self.executions.produced(producer);
                if self.intersects(context, when) {
                    available
                        .entry(name.clone())
                        .or_default()
                        .push((producer, when));
                }
            }
        }
        let mut wires = Vec::new();
        for (name, producers) in available {
            let present = producers.iter().fold(NEVER, |sum, &(_, when)| {
                self.executions.conditions.or(sum, when)
            });
            let producers: BTreeSet<_> = producers
                .into_iter()
                .map(|(producer, _)| producer)
                .collect();
            if !self.covers(context, present)
                || producers.len() < 2
                || producers.iter().all(|producer| match producer {
                    ProducerId::FlowInput(_) => true,
                    ProducerId::BlockOutput { block, .. } => outside.contains(block),
                })
            {
                continue;
            }
            wires.push((
                *producers.first().expect("the producers are nonempty"),
                name,
            ));
            self.classes.push(producers);
        }
        wires.sort();
        wires.into_iter().map(|(_, name)| name).collect()
    }
}

impl Executions {
    pub(super) fn verify(
        &mut self,
        flow: &Flow,
        plan: &ExecutionPlan,
        merges: &[WireMerge],
    ) -> bool {
        if !matches!(plan, ExecutionPlan::End { index, .. } if *index == flow.blocks.len() - 1) {
            return false;
        }
        let order = crate::plan::verify::emitted(plan);
        if (0..flow.blocks.len() - 1).any(|block| {
            let count = order.iter().filter(|&&found| found == block).count();
            count != 1
                && !(count == 0
                    && matches!(flow.kind, crate::FlowKind::Preparation)
                    && flow.blocks[block].transition_target.is_some())
        }) {
            return false;
        }
        for (position, &first) in order.iter().enumerate() {
            for &second in &order[position + 1..] {
                if first > second {
                    let reversed = self.conditions.and(self.runs[first], self.runs[second]);
                    if self.has(reversed) {
                        return false;
                    }
                }
            }
        }
        let mut replay = Replay {
            executions: self,
            flow,
            merges,
            available: flow
                .flow_inputs
                .iter()
                .enumerate()
                .map(|(index, _)| (ProducerId::FlowInput(index), ALWAYS))
                .collect(),
            ran: vec![NEVER; flow.blocks.len()],
            returned: BTreeMap::new(),
            yields: BTreeMap::new(),
            transfers: BTreeMap::new(),
            scopes: Vec::new(),
            loop_indices: Vec::new(),
            exports: BTreeMap::new(),
            valid: true,
        };
        let domain = replay.executions.domain;
        replay.visit(plan, domain);
        if !replay.valid || replay.yields.values().any(|&when| when != NEVER) {
            return false;
        }
        for block in 0..flow.blocks.len() - 1 {
            let differs = replay
                .executions
                .conditions
                .xor(replay.ran[block], replay.executions.runs[block]);
            if replay.executions.has(differs) {
                return false;
            }
        }
        for (&outcome, &when) in &replay.executions.outcomes.clone() {
            let actual = replay.returned.get(&outcome).copied().unwrap_or(NEVER);
            let differs = replay.executions.conditions.xor(when, actual);
            if replay.executions.has(differs) {
                return false;
            }
        }
        true
    }
}

use super::condition::ALWAYS;
use crate::ExecutionOutcome;

struct Replay<'a> {
    executions: &'a mut Executions,
    flow: &'a Flow,
    merges: &'a [WireMerge],
    available: BTreeMap<ProducerId, Condition>,
    ran: Vec<Condition>,
    returned: BTreeMap<ExecutionOutcome, Condition>,
    yields: BTreeMap<(usize, usize), Condition>,
    transfers: BTreeMap<(usize, usize, Ident), Condition>,
    scopes: Vec<usize>,
    loop_indices: Vec<usize>,
    exports: BTreeMap<usize, Condition>,
    valid: bool,
}

impl Replay<'_> {
    fn requires(&mut self, context: Condition, when: Condition) {
        if self.executions.conditions.minus(context, when) != NEVER {
            self.valid = false;
        }
    }

    fn name(&self, producer: ProducerId) -> &Ident {
        match producer {
            ProducerId::FlowInput(index) => &self.flow.flow_inputs[index],
            ProducerId::BlockOutput { block, output } => &self.flow.blocks[block].outputs[output],
        }
    }

    fn present(&mut self, name: &Ident) -> Condition {
        let producers: Vec<_> = self
            .available
            .iter()
            .filter(|(producer, _)| self.name(**producer) == name)
            .map(|(_, &when)| when)
            .collect();
        producers
            .into_iter()
            .fold(NEVER, |sum, when| self.executions.conditions.or(sum, when))
    }

    fn enter(&mut self, block: usize, context: Condition, kind: BlockKind) {
        if self.flow.blocks[block].kind != kind {
            self.valid = false;
            return;
        }
        self.requires(context, self.executions.runs[block]);
        let repeated = self.executions.conditions.and(context, self.ran[block]);
        if repeated != NEVER {
            self.valid = false;
        }
        for (&dependency, &when) in &self.executions.dependencies.clone() {
            if dependency.capture.block != block {
                continue;
            }
            let needed = self.executions.conditions.and(context, when);
            let bound = self
                .available
                .get(&dependency.producer)
                .copied()
                .unwrap_or(NEVER);
            self.requires(needed, bound);
        }
        for merge in self
            .merges
            .iter()
            .filter(|merge| merge.after.contains(&block))
        {
            for &before in &merge.before {
                let needed = self
                    .executions
                    .conditions
                    .and(context, self.executions.runs[before]);
                self.requires(needed, self.ran[before]);
            }
            for &producer in &merge.producers {
                if let ProducerId::BlockOutput { block, .. } = producer {
                    let needed = self
                        .executions
                        .conditions
                        .and(context, self.executions.produced(producer));
                    self.requires(needed, self.ran[block]);
                }
            }
        }
        self.ran[block] = self.executions.conditions.or(self.ran[block], context);
        if kind != BlockKind::Loop {
            self.produce(block, context);
        }
    }

    fn produce(&mut self, block: usize, context: Condition) {
        for output in 0..self.flow.blocks[block].outputs.len() {
            let produced = if self.flow.blocks[block].branch_count() > 0 {
                let selected = self.executions.selected(block, output);
                self.executions.conditions.and(context, selected)
            } else {
                context
            };
            let entry = self
                .available
                .entry(ProducerId::BlockOutput { block, output })
                .or_insert(NEVER);
            *entry = self.executions.conditions.or(*entry, produced);
        }
    }

    #[allow(clippy::too_many_lines)] // Replays the complete branch and join scopes together.
    fn visit(&mut self, plan: &ExecutionPlan, context: Condition) {
        if context == NEVER {
            return;
        }
        match plan {
            ExecutionPlan::End { body, .. } => self.visit(body, context),
            ExecutionPlan::Action { index, next } | ExecutionPlan::Call { index, next } => {
                let kind = if matches!(plan, ExecutionPlan::Call { .. }) {
                    BlockKind::Call
                } else {
                    BlockKind::Action
                };
                self.enter(*index, context, kind);
                self.visit(next, context);
            }
            ExecutionPlan::Return { index } => {
                self.enter(*index, context, BlockKind::Return);
                let entry = self
                    .returned
                    .entry(ExecutionOutcome::Return {
                        block_index: *index,
                    })
                    .or_insert(NEVER);
                *entry = self.executions.conditions.or(*entry, context);
            }
            ExecutionPlan::Yield { wires, join } => {
                if !self.scopes.contains(&join.block) {
                    self.valid = false;
                }
                for wire in wires {
                    let present = self.present(wire);
                    self.requires(context, present);
                    let entry = self
                        .transfers
                        .entry((join.block, join.join, wire.clone()))
                        .or_insert(NEVER);
                    *entry = self.executions.conditions.or(*entry, context);
                }
                let entry = self.yields.entry((join.block, join.join)).or_insert(NEVER);
                *entry = self.executions.conditions.or(*entry, context);
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
            } => {
                let kind = if matches!(plan, ExecutionPlan::Choice { .. }) {
                    BlockKind::Choice
                } else {
                    BlockKind::Question
                };
                self.enter(*index, context, kind);
                self.branches(*index, branches, joins, context);
            }
            ExecutionPlan::Loop {
                index,
                body,
                branches,
                joins,
            } => loop_block::replay(self, *index, body, branches, joins, context),
            ExecutionPlan::Export { index, target } => {
                loop_block::export(self, *index, *target, context);
            }
            ExecutionPlan::Continue { index } => loop_block::repeat(self, *index, context),
        }
    }
    fn branches(&mut self, index: usize, branches: &[Branch], joins: &[Join], context: Condition) {
        let index = &index;
        let possible: Vec<_> = self
            .available
            .clone()
            .into_iter()
            .filter(|&(producer, when)| {
                !matches!(producer, ProducerId::BlockOutput { block, .. } if block == *index)
                    && self.executions.conditions.and(context, when) != NEVER
            })
            .map(|(producer, _)| producer)
            .collect();
        let outside: BTreeSet<_> = possible
            .into_iter()
            .map(|producer| self.name(producer).clone())
            .collect();
        if branches.len() != self.flow.blocks[*index].outputs.len() {
            self.valid = false;
            return;
        }
        self.scopes.push(*index);
        for (branch, subtree) in branches.iter().enumerate() {
            let selected = self.executions.selected(*index, branch);
            let branch_context = self.executions.conditions.and(context, selected);
            self.visit(&subtree.plan, branch_context);
        }
        for (join, continuation) in joins.iter().enumerate() {
            let yielded = self.yields.remove(&(*index, join)).unwrap_or(NEVER);
            let mut members = NEVER;
            for &branch in &continuation.branches {
                let selected = self.executions.selected(*index, branch);
                members = self.executions.conditions.or(members, selected);
            }
            self.requires(yielded, members);
            for wire in &continuation.wires {
                let present = self.present(wire);
                self.requires(yielded, present);
                let transferred = self
                    .transfers
                    .get(&(*index, join, wire.clone()))
                    .copied()
                    .unwrap_or(NEVER);
                self.requires(yielded, transferred);
            }
            for producer in self.available.keys().copied().collect::<Vec<_>>() {
                let name = self.name(producer);
                if !outside.contains(name) && !continuation.wires.contains(name) {
                    let bound = self.available[&producer];
                    let retained = self.executions.conditions.minus(bound, yielded);
                    self.available.insert(producer, retained);
                }
            }
            self.visit(&continuation.next, yielded);
        }
        self.scopes.pop();
    }
}
