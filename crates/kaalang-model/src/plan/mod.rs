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

mod choice;
mod question;
pub(crate) mod verify;
mod while_loop;

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
        "every execution of a validated flow reaches end"
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
        gates: builder.gates(executions),
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
    /// values a join carries. The end block's capture forms one more class.
    classes: Vec<BTreeSet<ProducerId>>,
}

/// One lowered subtree and what the executions passing through it do next.
struct Lowered<'e> {
    plan: ExecutionPlan,
    /// The executions that leave through a yield rather than end.
    yielding: Vec<&'e Execution>,
    /// Every computational block the subtree emits.
    emitted: BTreeSet<usize>,
}

/// A question or choice enclosing the code being lowered, with the shared
/// blocks each of its joins runs. Every block a branch may not run itself
/// belongs to exactly one join of one enclosing scope.
#[derive(Clone)]
struct Scope {
    block: usize,
    groups: Vec<BTreeSet<usize>>,
    /// The first join a yield from here may target. A branch may enter any of
    /// them; a join's own continuation may only hand on to a later one, which
    /// keeps the chain of nested joins acyclic.
    from: usize,
}

/// The producers one block captures in one execution.
fn producers(execution: &Execution, block: usize) -> impl Iterator<Item = ProducerId> + '_ {
    execution
        .dependencies
        .iter()
        .filter(move |dependency| dependency.capture.block == block)
        .map(|dependency| dependency.producer)
}

/// A block is settled once its producers and the participating branch-local
/// work before every merge it captures have run.
fn settled(
    merges: &[WireMerge],
    execution: &Execution,
    block: usize,
    done: &BTreeSet<usize>,
) -> bool {
    producers(execution, block).all(|producer| match producer {
        ProducerId::FlowInput(_) => true,
        ProducerId::BlockOutput { block, .. } => done.contains(&block),
    }) && merges
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
                    ProducerId::FlowInput(_) => true,
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
        let candidates = (0..self.end)
            .filter(|block| !done.contains(block) && !forbidden.contains(block))
            .filter(|&block| {
                executions.iter().all(|execution| {
                    execution.participates(block) && settled(self.merges, execution, block, done)
                })
            })
            .collect::<Vec<_>>();
        // Source order is the execution order, so the next block to emit is the
        // first one every execution here still has to run.
        let Some(&block) = candidates.first() else {
            return self.leaf(executions, done, forbidden, scopes);
        };
        let mut next_done = done.clone();
        next_done.insert(block);
        if self.flow.blocks[block].kind == BlockKind::Action {
            let mut next = self.lower(executions, &next_done, forbidden, scopes)?;
            next.emitted.insert(block);
            return Ok(Lowered {
                plan: ExecutionPlan::Action {
                    index: block,
                    next: Box::new(next.plan),
                },
                yielding: next.yielding,
                emitted: next.emitted,
            });
        }
        if self.flow.blocks[block].kind == BlockKind::While {
            return while_loop::lower(self, block, executions, &next_done, forbidden, scopes);
        }
        self.branch(block, executions, &next_done, forbidden, scopes)
    }

    /// No block can run here: the executions yield to the innermost enclosing
    /// join whose shared computation is still pending, or arrive at end. A
    /// block pending in only some of them has no place in the branch tree.
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
            let [wire] = self.flow.blocks[self.end].inputs.as_slice() else {
                unreachable!("the end block captures exactly one wire")
            };
            return Ok(Lowered {
                plan: ExecutionPlan::EndArrival {
                    wire: wire.ident.clone(),
                },
                yielding: Vec::new(),
                emitted,
            });
        }

        let join = target.expect("every shared block belongs to a join of an enclosing scope");
        // Every wire available here, spelled by its own producer so a type
        // error names the authored occurrence. The join keeps the ones it
        // carries.
        let wires = self.available(executions[0], done).into_keys().collect();
        Ok(Lowered {
            plan: if self.flow.blocks[join.block].kind == BlockKind::While {
                ExecutionPlan::Repeat { index: join.block }
            } else {
                ExecutionPlan::Yield { wires, join }
            },
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
            BlockKind::Action | BlockKind::End | BlockKind::While => {
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
    /// the join of exactly those branches, never inside one of them. Groups the
    /// shared blocks by that set of branches, innermost first: a contained
    /// branch set is strictly smaller, so it joins before the one containing it.
    fn groups(
        &self,
        block: usize,
        selections: &[Vec<&Execution>],
        done: &BTreeSet<usize>,
        forbidden: &BTreeSet<usize>,
    ) -> Result<Vec<Group>, Unstructured> {
        let mut groups = BTreeMap::<Vec<usize>, BTreeSet<usize>>::new();
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
                groups.entry(members).or_default().insert(candidate);
            }
        }
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by(|(left, _), (right, _)| {
            left.len().cmp(&right.len()).then_with(|| left.cmp(right))
        });
        if self.flow.blocks[block].kind == BlockKind::Choice && !choice::joinable(&groups) {
            return Err(Unstructured);
        }
        Ok(groups)
    }

    /// Lowers one join per group of branches that share computation, innermost
    /// first. The executions that wait for a group's blocks yield its wires;
    /// the shared continuation then runs once. A contained group's continuation
    /// hands its value to the group containing it, so the later group's blocks
    /// stay forbidden inside the earlier continuation and reachable from it as
    /// a further join of this block. Returns the joins and the executions that
    /// leave their continuations toward an enclosing join.
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
                    ProducerId::FlowInput(_) => true,
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
        let outputs = done
            .iter()
            .filter(|&&block| execution.participates(block))
            .flat_map(|&block| {
                let selected = execution.selected(block);
                self.flow.blocks[block]
                    .outputs
                    .iter()
                    .enumerate()
                    .filter(move |(output, _)| selected.is_none_or(|branch| branch == *output))
                    .map(move |(output, name)| (name, ProducerId::BlockOutput { block, output }))
            });
        flow_inputs
            .chain(outputs)
            .map(|(name, producer)| (name.clone(), producer))
            .collect()
    }

    /// Logical names produced more than once that no single binding unifies.
    /// Rust would otherwise never compare the types of such alternatives.
    fn gates(&self, executions: &[Execution]) -> Vec<Ident> {
        let mut classes = self.classes.clone();
        classes.push(
            executions
                .iter()
                .flat_map(|execution| producers(execution, self.end))
                .collect(),
        );
        let mut occurrences = BTreeMap::<&Ident, BTreeSet<ProducerId>>::new();
        for (block, declaration) in self.flow.blocks.iter().enumerate() {
            for (output, name) in declaration.outputs.iter().enumerate() {
                occurrences
                    .entry(name)
                    .or_default()
                    .insert(ProducerId::BlockOutput { block, output });
            }
        }
        occurrences
            .into_iter()
            .filter(|(_, producers)| {
                producers.len() >= 2 && !classes.iter().any(|class| producers.is_subset(class))
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Narrows every yield into one join to the wires that join carries, keeping
/// each yield's own producer spellings.
fn fill_yields(plan: &mut ExecutionPlan, wires: &[Ident], target: JoinTarget) {
    match plan {
        ExecutionPlan::While { body, next, .. } => {
            fill_yields(body, wires, target);
            fill_yields(next, wires, target);
        }
        ExecutionPlan::Action { next, .. } | ExecutionPlan::End { body: next, .. } => {
            fill_yields(next, wires, target);
        }
        ExecutionPlan::Question { branches, join, .. } => {
            for branch in branches {
                fill_yields(&mut branch.plan, wires, target);
            }
            if let Some(join) = join {
                fill_yields(&mut join.next, wires, target);
            }
        }
        ExecutionPlan::Choice {
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
        ExecutionPlan::EndArrival { .. }
        | ExecutionPlan::Yield { .. }
        | ExecutionPlan::Repeat { .. } => {}
    }
}
