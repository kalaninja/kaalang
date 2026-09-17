//! Builds the lowering plan over the validated source order: which blocks run
//! inside a branch, which run once after branches join, and which alternative
//! values each join carries.
//!
//! Joins are lowering structure. Execution validation owns semantic convergence;
//! the branch rule of RFC 0001 §7 is what guarantees a nested branch tree exists.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    BlockKind, Branch, BranchSelection, Execution, ExecutionPlan, Flow, Join, JoinTarget,
    ProducerId, WireMerge,
};

mod break_block;
mod choice;
mod loop_block;
mod question;
mod return_block;
pub(crate) mod verify;

/// Builds the nested branch tree the validated flow lowers to. Every accepted
/// flow has one, so a failure here is a compiler bug rather than a rejected
/// program.
pub(crate) fn flow(flow: &Flow, executions: &[Execution], merges: &[WireMerge]) -> ExecutionPlan {
    let end = flow.blocks.len() - 1;
    let mut builder = Builder {
        flow,
        end,
        merges,
        classes: Vec::new(),
    };
    let all = executions.iter().collect::<Vec<_>>();
    let lowered = builder
        .lower(&all, &BTreeSet::new(), &BTreeSet::new(), &[])
        .expect("a validated flow lowers to nested branches");
    assert!(
        lowered.yielding.is_empty(),
        "every execution has a final outcome"
    );
    // A wrong plan would silently reorder effects, so fail the expansion
    // instead of emitting it.
    assert!(
        verify::plan(flow, &lowered.plan, executions, merges),
        "the lowered plan replays every execution and respects its wire merges"
    );
    ExecutionPlan::End {
        index: end,
        body: Box::new(lowered.plan),
        gates: builder.gates(),
    }
}

/// The branch tree cannot express this part of the flow while emitting every
/// body once. The branch rule rules that out for an accepted flow, so reaching
/// it marks a compiler bug.
#[derive(Debug)]
struct Unstructured;

/// The branches whose executions share a set of blocks, with those blocks.
type Group = (Vec<usize>, BTreeSet<usize>);

struct Builder<'a> {
    flow: &'a Flow,
    end: usize,
    merges: &'a [WireMerge],
    /// Producer occurrences that one Rust binding unifies: the alternative
    /// values a join carries.
    classes: Vec<BTreeSet<ProducerId>>,
}

/// One lowered subtree and what the executions passing through it do next.
struct Lowered<'e> {
    plan: ExecutionPlan,
    /// The executions that leave through a yield rather than end.
    yielding: Vec<&'e Execution>,
    /// Every authored block the subtree emits.
    emitted: BTreeSet<usize>,
}

/// An enclosing selection or loop, with the blocks its joins or normal exit
/// run. Cycle scopes also mark iteration boundaries without a pending successor.
#[derive(Clone)]
struct Scope {
    block: usize,
    groups: Vec<BTreeSet<usize>>,
    /// The first join a yield from here may target. A branch may enter any of
    /// them; a join's own continuation may only hand on to a later one, which
    /// keeps the chain of nested joins acyclic.
    from: usize,
}

/// A block is settled once its producers and the participating branch-local
/// work before every merge it captures have run.
fn settled(
    merges: &[WireMerge],
    execution: &Execution,
    block: usize,
    done: &BTreeSet<usize>,
) -> bool {
    execution
        .dependencies
        .iter()
        .filter(|dependency| dependency.capture.block == block)
        .all(|dependency| match dependency.producer {
            ProducerId::FlowInput(_) | ProducerId::CycleInput { .. } => true,
            ProducerId::BlockOutput { block, .. } => done.contains(&block),
        })
        && merges
            .iter()
            .filter(|merge| merge.after.contains(&block))
            .all(|merge| {
                merge
                    .before
                    .iter()
                    .all(|before| !execution.participates(*before) || done.contains(before))
                    && merge.producers.iter().all(|producer| match producer {
                        ProducerId::BlockOutput { block, .. } => {
                            !execution.participates(*block) || done.contains(block)
                        }
                        ProducerId::FlowInput(_) | ProducerId::CycleInput { .. } => true,
                    })
            })
}

impl Builder<'_> {
    fn lower<'e>(
        &mut self,
        executions: &[&'e Execution],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Result<Lowered<'e>, Unstructured> {
        debug_assert!(
            !executions.is_empty(),
            "every branch selection has an execution"
        );
        // Source order is the execution order, so the next block to emit is the
        // first one every execution here still has to run.
        let Some(block) = (0..self.end)
            .filter(|block| !done.contains(block) && !forbidden.contains(block))
            .find(|&block| {
                executions.iter().all(|execution| {
                    execution.participates(block) && settled(self.merges, execution, block, done)
                })
            })
        else {
            return self.leaf(executions, done, forbidden, scopes);
        };
        let mut next_done = done.clone();
        next_done.insert(block);
        // An action and a call each run in place and hand over every output.
        let kind = self.flow.blocks[block].kind;
        if matches!(kind, BlockKind::Action | BlockKind::Call) {
            let mut next = self.lower(executions, &next_done, forbidden, scopes)?;
            next.emitted.insert(block);
            let index = block;
            let plan = Box::new(next.plan);
            return Ok(Lowered {
                plan: if kind == BlockKind::Call {
                    ExecutionPlan::Call { index, next: plan }
                } else {
                    ExecutionPlan::Action { index, next: plan }
                },
                yielding: next.yielding,
                emitted: next.emitted,
            });
        }
        if self.flow.blocks[block].kind == BlockKind::Loop {
            return loop_block::lower(self, block, executions, &next_done, forbidden, scopes);
        }
        if self.flow.blocks[block].kind == BlockKind::Break {
            return Ok(break_block::lower(self.flow, block));
        }
        if self.flow.blocks[block].kind == BlockKind::Return {
            return Ok(return_block::lower(block));
        }
        self.branch(block, executions, &next_done, forbidden, scopes)
    }

    /// No block can run here: the executions yield to the innermost enclosing
    /// join whose shared computation is still pending, repeat the enclosing
    /// iteration, or arrive at end. A block pending in only some of them has no
    /// place in the branch tree.
    fn leaf<'e>(
        &self,
        executions: &[&'e Execution],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Result<Lowered<'e>, Unstructured> {
        let pending = |block: usize| {
            !done.contains(&block)
                && executions
                    .iter()
                    .any(|execution| execution.participates(block))
        };
        let waiting = forbidden
            .iter()
            .copied()
            .filter(|&block| pending(block))
            .collect::<BTreeSet<_>>();
        let target = scopes.iter().rev().find_map(|scope| {
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
        });
        let all_wait = !waiting.is_empty()
            && executions
                .iter()
                .all(|execution| waiting.iter().any(|&block| execution.participates(block)));
        if (0..self.end).any(|block| !forbidden.contains(&block) && pending(block))
            || (!waiting.is_empty() && !all_wait)
        {
            return Err(Unstructured);
        }
        let emitted = BTreeSet::new();
        if waiting.is_empty() {
            // A body repeats only its own innermost active iteration.
            if let Some(scope) = scopes
                .iter()
                .rev()
                .find(|scope| self.flow.blocks[scope.block].loop_end.is_some())
                && executions
                    .iter()
                    .all(|execution| execution.repeats.contains(&scope.block))
            {
                return Ok(Lowered {
                    plan: ExecutionPlan::Repeat { index: scope.block },
                    yielding: Vec::new(),
                    emitted,
                });
            }
            return Err(Unstructured);
        }

        let join = target.expect("every shared block belongs to a join of an enclosing scope");
        // Every wire available here, spelled by its own producer so a type
        // error names the authored occurrence. The join keeps the ones it
        // carries.
        let wires = self.available(executions[0], done).into_keys().collect();
        Ok(Lowered {
            plan: ExecutionPlan::Yield { wires, join },
            yielding: executions.to_vec(),
            emitted,
        })
    }

    /// Lowers a question or choice: exclusive computation inside each branch,
    /// shared computation once after the join that its branches yield into.
    fn branch<'e>(
        &mut self,
        block: usize,
        executions: &[&'e Execution],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Result<Lowered<'e>, Unstructured> {
        let selections = (0..self.flow.blocks[block].outputs.len())
            .map(|branch| {
                let selection = BranchSelection { block, branch };
                executions
                    .iter()
                    .copied()
                    .filter(|execution| execution.branches.binary_search(&selection).is_ok())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let groups = self.groups(block, &selections, done, forbidden)?;
        let shared = groups
            .iter()
            .flat_map(|(_, blocks)| blocks.iter().copied())
            .collect::<BTreeSet<_>>();
        let inner_forbidden = forbidden.union(&shared).copied().collect::<BTreeSet<_>>();
        let mut inner_scopes = scopes.to_vec();
        inner_scopes.push(Scope {
            block,
            groups: groups.iter().map(|(_, blocks)| blocks.clone()).collect(),
            from: 0,
        });
        let mut branches = selections
            .iter()
            .map(|selection| self.lower(selection, done, &inner_forbidden, &inner_scopes))
            .collect::<Result<Vec<_>, Unstructured>>()?;

        let mut emitted = BTreeSet::from([block]);
        for branch in &branches {
            emitted.extend(&branch.emitted);
        }
        let (joins, mut yielding) = self.join(
            block,
            &groups,
            &mut branches,
            &mut emitted,
            done,
            forbidden,
            scopes,
        )?;
        // A yield that waits for no join of this block passes it toward an
        // enclosing one.
        for branch in &branches {
            yielding.extend(branch.yielding.iter().copied().filter(|execution| {
                !shared
                    .iter()
                    .any(|&candidate| execution.participates(candidate))
            }));
        }

        let branches = branches
            .into_iter()
            .map(|branch| Branch {
                plan: Box::new(branch.plan),
            })
            .collect::<Vec<_>>();

        let plan = match self.flow.blocks[block].kind {
            BlockKind::Question => question::dispatch(block, branches, joins),
            BlockKind::Choice => choice::dispatch(block, branches, joins),
            BlockKind::Action
            | BlockKind::Call
            | BlockKind::End
            | BlockKind::Loop
            | BlockKind::Break
            | BlockKind::Return => {
                unreachable!("only questions and choices branch")
            }
        };
        Ok(Lowered {
            plan,
            yielding,
            emitted,
        })
    }

    /// Blocks that two or more branches run are shared: they run once after
    /// the join of exactly those branches, never inside one of them. The same
    /// branches may join repeatedly as a partial execution context widens;
    /// later narrowing belongs to a selection inside that continuation.
    fn groups(
        &self,
        block: usize,
        selections: &[Vec<&Execution>],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
    ) -> Result<Vec<Group>, Unstructured> {
        let mut groups = BTreeMap::<Vec<usize>, Vec<(BTreeSet<usize>, BTreeSet<usize>)>>::new();
        for candidate in
            (0..self.end).filter(|block| !done.contains(block) && !forbidden.contains(block))
        {
            let members = (0..selections.len())
                .filter(|&branch| {
                    selections[branch]
                        .iter()
                        .any(|execution| execution.participates(candidate))
                })
                .collect::<Vec<_>>();
            if members.len() >= 2 {
                let context = selections
                    .iter()
                    .flatten()
                    .enumerate()
                    .filter_map(|(execution, run)| run.participates(candidate).then_some(execution))
                    .collect::<BTreeSet<_>>();
                let groups = groups.entry(members).or_default();
                if let Some((scope, blocks)) = groups.last_mut()
                    && context.is_subset(scope)
                {
                    blocks.insert(candidate);
                } else {
                    groups.push((context, BTreeSet::from([candidate])));
                }
            }
        }
        let mut groups = groups
            .into_iter()
            .flat_map(|(branches, groups)| {
                groups
                    .into_iter()
                    .map(move |(_, blocks)| (branches.clone(), blocks))
            })
            .collect::<Vec<_>>();
        // The map already orders by branches, then by insertion; the stable
        // sort only moves narrower groups first.
        groups.sort_by_key(|(branches, _)| branches.len());
        if self.flow.blocks[block].kind == BlockKind::Choice && !choice::joinable(&groups) {
            return Err(Unstructured);
        }
        Ok(groups)
    }

    /// Lowers shared groups innermost first. Each continuation runs once and
    /// may yield to a wider group; later groups' blocks stay excluded until then.
    /// Returns joins and executions yielding to an enclosing selection.
    #[allow(clippy::too_many_arguments)]
    fn join<'e>(
        &mut self,
        block: usize,
        groups: &[Group],
        branches: &mut [Lowered<'e>],
        emitted: &mut BTreeSet<usize>,
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
        scopes: &[Scope],
    ) -> Result<(Vec<Join>, Vec<&'e Execution>), Unstructured> {
        let mut joined = done.clone();
        joined.extend(emitted.iter());
        let mut joins = Vec::<Join>::with_capacity(groups.len());
        let mut yielding = Vec::new();
        for (join, (members, blocks)) in groups.iter().enumerate() {
            let executions = members
                .iter()
                .flat_map(|&branch| branches[branch].yielding.iter().copied())
                .filter(|execution| {
                    blocks
                        .iter()
                        .any(|&candidate| execution.participates(candidate))
                })
                .collect::<Vec<_>>();
            let wires = self.join_wires(&executions, &joined, done);
            let target = JoinTarget { block, join };
            // A yield into this join sits in a member branch or in the
            // continuation of a group this one contains.
            for branch in branches.iter_mut() {
                fill_yields(&mut branch.plan, &wires, target);
            }
            for earlier in &mut joins {
                fill_yields(&mut earlier.next, &wires, target);
            }
            let later = groups[join + 1..]
                .iter()
                .flat_map(|(_, blocks)| blocks.iter().copied())
                .collect::<BTreeSet<_>>();
            let inner_forbidden = forbidden.union(&later).copied().collect::<BTreeSet<_>>();
            let mut inner_scopes = scopes.to_vec();
            inner_scopes.push(Scope {
                block,
                groups: groups.iter().map(|(_, blocks)| blocks.clone()).collect(),
                from: join + 1,
            });
            let next = self.lower(&executions, &joined, &inner_forbidden, &inner_scopes)?;
            emitted.extend(&next.emitted);
            joined.extend(&next.emitted);
            // An execution the continuation hands to a later join of this block
            // has not left the brancher.
            yielding.extend(
                next.yielding
                    .into_iter()
                    .filter(|execution| !later.iter().any(|&block| execution.participates(block))),
            );
            joins.push(Join {
                branches: members.clone(),
                wires,
                next: Box::new(next.plan),
            });
        }
        Ok((joins, yielding))
    }

    /// Carries every merged wire not already bound outside this brancher,
    /// including unused wires whose values must live in the shared scope.
    /// Producer history from an earlier join does not require another transfer.
    fn join_wires(
        &mut self,
        executions: &[&Execution],
        done: &BTreeSet<usize>,
        outside: &BTreeSet<usize>,
    ) -> Vec<Ident> {
        let available = executions
            .iter()
            .map(|execution| self.available(execution, done))
            .collect::<Vec<_>>();
        let Some(first) = available.first() else {
            return Vec::new();
        };
        let mut wires = Vec::new();
        for name in first.keys() {
            let Some(producers) = available
                .iter()
                .map(|available| available.get(name).copied())
                .collect::<Option<BTreeSet<_>>>()
            else {
                continue;
            };
            let Some(&earliest) = producers.first() else {
                continue;
            };
            if producers.len() < 2
                || producers.iter().all(|producer| match producer {
                    ProducerId::FlowInput(_) | ProducerId::CycleInput { .. } => true,
                    ProducerId::BlockOutput { block, .. } => outside.contains(block),
                })
            {
                continue;
            }
            self.classes.push(producers);
            wires.push((earliest, name.clone()));
        }
        wires.sort();
        wires.into_iter().map(|(_, name)| name).collect()
    }

    /// Every wire a block may still capture in one execution once `done` ran:
    /// provided by a flow input or a done block. A bare capture does not remove
    /// one; Rust reports a use after a move.
    fn available(
        &self,
        execution: &Execution,
        done: &BTreeSet<usize>,
    ) -> BTreeMap<Ident, ProducerId> {
        let flow_inputs = self
            .flow
            .flow_inputs
            .iter()
            .enumerate()
            .map(|(index, name)| (name, ProducerId::FlowInput(index)));
        let outputs = done.iter().flat_map(|&block| {
            self.flow.blocks[block]
                .outputs
                .iter()
                .enumerate()
                .filter(move |(output, _)| {
                    self.flow.produces(
                        execution,
                        ProducerId::BlockOutput {
                            block,
                            output: *output,
                        },
                    )
                })
                .map(move |(output, name)| (name, ProducerId::BlockOutput { block, output }))
        });
        flow_inputs
            .chain(outputs)
            .map(|(name, producer)| (name.clone(), producer))
            .collect()
    }

    /// Logical names produced more than once that no single binding unifies.
    /// Rust would otherwise never compare the types of such alternatives.
    fn gates(&self) -> Vec<Ident> {
        self.merges
            .iter()
            .filter(|merge| {
                !self.classes.iter().any(|class| {
                    merge
                        .producers
                        .iter()
                        .all(|producer| class.contains(producer))
                })
            })
            .map(|merge| merge.wire.clone())
            .collect()
    }
}

/// Each body occurs once in the verified plan. Visiting branches before their
/// joins gives the source order when filtered by an execution's participants,
/// including branches yielding to an outer join.
pub(crate) fn serial_order(plan: &ExecutionPlan, order: &mut Vec<usize>) {
    match plan {
        ExecutionPlan::Loop { index, body, next } => {
            order.push(*index);
            serial_order(body, order);
            if let Some(next) = next {
                serial_order(next, order);
            }
        }
        ExecutionPlan::Break { index, .. } | ExecutionPlan::Return { index } => {
            order.push(*index);
        }
        ExecutionPlan::Action { index, next } | ExecutionPlan::Call { index, next } => {
            order.push(*index);
            serial_order(next, order);
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
            order.push(*index);
            for branch in branches {
                serial_order(&branch.plan, order);
            }
            for join in joins {
                serial_order(&join.next, order);
            }
        }
        ExecutionPlan::End { index, body, .. } => {
            serial_order(body, order);
            order.push(*index);
        }
        ExecutionPlan::Yield { .. } | ExecutionPlan::Repeat { .. } => {}
    }
}

/// Narrows every yield into one join to the wires that join carries, keeping
/// each yield's own producer spellings.
fn fill_yields(plan: &mut ExecutionPlan, wires: &[Ident], target: JoinTarget) {
    match plan {
        ExecutionPlan::Loop { body, next, .. } => {
            fill_yields(body, wires, target);
            if let Some(next) = next {
                fill_yields(next, wires, target);
            }
        }
        ExecutionPlan::Action { next, .. }
        | ExecutionPlan::Call { next, .. }
        | ExecutionPlan::End { body: next, .. } => {
            fill_yields(next, wires, target);
        }
        ExecutionPlan::Question {
            branches, joins, ..
        }
        | ExecutionPlan::Choice {
            branches, joins, ..
        } => {
            for branch in branches {
                fill_yields(&mut branch.plan, wires, target);
            }
            for join in joins {
                fill_yields(&mut join.next, wires, target);
            }
        }
        ExecutionPlan::Yield {
            wires: available,
            join,
        } if *join == target => {
            *available = wires
                .iter()
                .map(|wire| {
                    available
                        .iter()
                        .find(|candidate| *candidate == wire)
                        .cloned()
                        .expect("a join carries only wires available on every yield")
                })
                .collect();
        }
        ExecutionPlan::Yield { .. }
        | ExecutionPlan::Repeat { .. }
        | ExecutionPlan::Break { .. }
        | ExecutionPlan::Return { .. } => {}
    }
}
