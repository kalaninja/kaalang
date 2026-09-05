//! Selects one permitted serial order of the recorded executions and builds the
//! lowering plan: which blocks run inside a branch, which run once after
//! branches join, and which alternative values each join carries.
//!
//! Joins are lowering structure. Execution validation owns semantic convergence;
//! a flow that cannot share every body in nested branches uses guarded blocks.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    BlockKind, Branch, BranchSelection, Execution, ExecutionPlan, Flow, Join, JoinTarget,
    ProducerId,
};

mod choice;
mod question;
mod verify;

/// Builds a lowering plan without imposing additional language restrictions:
/// the structured branch tree where it can share every body, the guarded
/// schedule otherwise.
pub(crate) fn flow(flow: &Flow, executions: &[Execution]) -> ExecutionPlan {
    let end = flow.blocks.len() - 1;
    let mut builder = Builder {
        flow,
        end,
        classes: Vec::new(),
    };
    let all = executions.iter().collect::<Vec<_>>();
    let structured = builder
        .lower(&all, &BTreeSet::new(), &BTreeSet::new(), &[])
        .ok()
        .filter(|lowered| lowered.yielding.is_empty())
        .filter(|lowered| {
            // A structured plan that fails the replay is a builder bug. Debug
            // builds report it; release builds keep the guarded schedule,
            // which stays correct.
            let verified = verify::plan(flow, &lowered.plan, executions);
            debug_assert!(verified, "a structured plan replays every execution");
            verified
        });
    let (body, gates) = match structured {
        Some(lowered) => (lowered.plan, builder.gates(executions)),
        None => (
            ExecutionPlan::Guarded {
                inputs: flow.flow_inputs.clone(),
                blocks: (0..end).collect(),
            },
            Vec::new(),
        ),
    };
    ExecutionPlan::End {
        index: end,
        body: Box::new(body),
        gates,
    }
}

/// The structured branch tree cannot express this part of the flow while
/// emitting every body once; the caller falls back to a guarded schedule. The
/// analyzer has already reported every language violation.
struct Unstructured;

/// The branches whose executions share a set of blocks, with those blocks.
type Group = (Vec<usize>, BTreeSet<usize>);

struct Builder<'a> {
    flow: &'a Flow,
    end: usize,
    /// Producer occurrences that one Rust binding unifies: the alternative
    /// values a join carries. End's captures form one more class.
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
}

/// The producers one block captures in one execution.
fn producers(execution: &Execution, block: usize) -> impl Iterator<Item = ProducerId> + '_ {
    execution
        .dependencies
        .iter()
        .filter(move |dependency| dependency.capture.block == block)
        .map(|dependency| dependency.producer)
}

/// A block is ready once every producer it captures in this execution ran.
fn ready(execution: &Execution, block: usize, done: &BTreeSet<usize>) -> bool {
    producers(execution, block).all(|producer| match producer {
        ProducerId::FlowInput(_) => true,
        ProducerId::BlockOutput { block, .. } => done.contains(&block),
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
                executions
                    .iter()
                    .all(|execution| execution.participates(block) && ready(execution, block, done))
            })
            .collect::<Vec<_>>();
        // Independent computation runs before a question or choice, so a
        // terminal branch never returns while participating work is pending.
        let action = candidates
            .iter()
            .find(|&&block| self.flow.blocks[block].kind == BlockKind::Action);
        if let Some(&block) = action {
            let mut done = done.clone();
            done.insert(block);
            let mut next = self.lower(executions, &done, forbidden, scopes)?;
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

        // Authored order breaks ties only between branchers whose continuations
        // can be shared. A later independent brancher may have to run first.
        // ponytail: failed orders may require factorial search; memoize failed
        // lowering states if large flows make this costly.
        let mut rejected = false;
        for block in candidates {
            let mut next_done = done.clone();
            next_done.insert(block);
            let classes = self.classes.len();
            if let Ok(lowered) = self.branch(block, executions, &next_done, forbidden, scopes) {
                return Ok(lowered);
            }
            self.classes.truncate(classes);
            rejected = true;
        }
        let structured = if rejected {
            Err(Unstructured)
        } else {
            self.leaf(executions, done, forbidden, scopes)
        };
        match structured {
            Err(Unstructured) if forbidden.is_empty() => {
                self.guarded(executions, done).ok_or(Unstructured)
            }
            result => result,
        }
    }

    /// Keeps structured prefixes in their Rust scopes and guards only a suffix
    /// whose existing inputs are available in every execution entering it.
    fn guarded<'e>(
        &self,
        executions: &[&'e Execution],
        done: &BTreeSet<usize>,
    ) -> Option<Lowered<'e>> {
        let available = executions
            .iter()
            .map(|execution| self.available(execution, done))
            .collect::<Vec<_>>();
        let mut inputs = BTreeSet::new();
        for (execution, current) in executions.iter().zip(&available) {
            for dependency in &execution.dependencies {
                if done.contains(&dependency.capture.block) {
                    continue;
                }
                let name = match dependency.producer {
                    ProducerId::FlowInput(input) => &self.flow.flow_inputs[input],
                    ProducerId::BlockOutput { block, output } if done.contains(&block) => {
                        &self.flow.blocks[block].outputs[output]
                    }
                    ProducerId::BlockOutput { .. } => continue,
                };
                if current.get(name) != Some(&dependency.producer)
                    || available.iter().any(|state| !state.contains_key(name))
                {
                    return None;
                }
                inputs.insert(name.clone());
            }
        }
        let blocks = (0..self.end)
            .filter(|block| {
                !done.contains(block)
                    && executions
                        .iter()
                        .any(|execution| execution.participates(*block))
            })
            .collect::<Vec<_>>();
        Some(Lowered {
            emitted: blocks.iter().copied().collect(),
            plan: ExecutionPlan::Guarded {
                inputs: inputs.into_iter().collect(),
                blocks,
            },
            yielding: Vec::new(),
        })
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
                .position(|group| !group.is_disjoint(&waiting))
                .map(|join| JoinTarget {
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
            let inputs = self.flow.blocks[self.end]
                .inputs
                .iter()
                .map(|input| input.ident.clone())
                .collect();
            return Ok(Lowered {
                plan: ExecutionPlan::EndArrival { inputs },
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
        });
        let mut branches = selections
            .iter()
            .map(|selection| self.lower(selection, done, &inner_forbidden, &inner_scopes))
            .collect::<Result<Vec<_>, Unstructured>>()?;

        let mut emitted = BTreeSet::from([block]);
        for branch in &branches {
            emitted.extend(&branch.emitted);
        }
        let (mut joins, mut yielding) = self.join(
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

        // An arm that reaches end returns outright when a sibling arm yields.
        // `join` left each join's flag at "its continuation yields".
        let arms_yield = !yielding.is_empty();
        for join in &mut joins {
            join.early_return = arms_yield && !join.early_return;
        }
        let any_branch_yields = branches.iter().any(|branch| !branch.yielding.is_empty());
        let branches = branches
            .into_iter()
            .map(|branch| Branch {
                early_return: any_branch_yields && branch.yielding.is_empty(),
                plan: Box::new(branch.plan),
            })
            .collect::<Vec<_>>();

        let plan = match self.flow.blocks[block].kind {
            BlockKind::Question => question::dispatch(block, branches, joins),
            BlockKind::Choice => choice::dispatch(block, branches, joins),
            BlockKind::Action | BlockKind::End => unreachable!("only questions and choices branch"),
        };
        Ok(Lowered {
            plan,
            yielding,
            emitted,
        })
    }

    /// Blocks that two or more branches run are shared: they run once after
    /// the join of exactly those branches, never inside one of them. Groups the
    /// shared blocks by that set of branches, ordered by first branch.
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
        let groups = groups.into_iter().collect::<Vec<_>>();
        if self.flow.blocks[block].kind == BlockKind::Choice && !choice::joinable(&groups) {
            return Err(Unstructured);
        }
        Ok(groups)
    }

    /// Lowers one join per group of branches that share computation. The
    /// executions that wait for a group's blocks yield its wires; the shared
    /// continuation then runs once. Returns the joins, each flagged with
    /// whether its continuation yields, and the executions that leave the
    /// continuations toward an enclosing join.
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
        let mut joins = Vec::with_capacity(groups.len());
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
            let wires = self.join_wires(&executions, &joined);
            let target = JoinTarget { block, join };
            for &branch in members {
                fill_yields(&mut branches[branch].plan, &wires, target);
            }
            let next = self.lower(&executions, &joined, forbidden, scopes)?;
            emitted.extend(&next.emitted);
            // Recorded as "the continuation yields" until `branch` knows every
            // sibling arm and settles the early return.
            let next_yields = !next.yielding.is_empty();
            yielding.extend(next.yielding);
            joins.push(Join {
                branches: members.clone(),
                wires,
                next: Box::new(next.plan),
                early_return: next_yields,
            });
        }
        Ok((joins, yielding))
    }

    /// The logical wires a join carries: available in every execution that
    /// reaches it, from producers that differ between those executions. A wire
    /// with one producer everywhere is already bound before the branch: a block
    /// emitted inside a branch runs only in that branch's executions, and a
    /// join has at least two member branches.
    fn join_wires(&mut self, executions: &[&Execution], done: &BTreeSet<usize>) -> Vec<Ident> {
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
            if producers.len() < 2 {
                continue;
            }
            self.classes.push(producers);
            wires.push((earliest, name.clone()));
        }
        wires.sort();
        wires.into_iter().map(|(_, name)| name).collect()
    }

    /// Every wire a block may still capture in one execution once `done` ran:
    /// provided by a flow input or a done block and not consumed by a done block.
    fn available(
        &self,
        execution: &Execution,
        done: &BTreeSet<usize>,
    ) -> BTreeMap<Ident, ProducerId> {
        let consumed = execution
            .dependencies
            .iter()
            .filter(|dependency| {
                let capture = dependency.capture;
                done.contains(&capture.block)
                    && !self.flow.blocks[capture.block].inputs[capture.input].borrowed
            })
            .map(|dependency| dependency.producer)
            .collect::<BTreeSet<_>>();
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
                let selected = execution
                    .branches
                    .iter()
                    .find(|selection| selection.block == block);
                self.flow.blocks[block]
                    .outputs
                    .iter()
                    .enumerate()
                    .filter(move |(output, _)| {
                        selected.is_none_or(|selection| selection.branch == *output)
                    })
                    .map(move |(output, name)| (name, ProducerId::BlockOutput { block, output }))
            });
        flow_inputs
            .chain(outputs)
            .filter(|(_, producer)| !consumed.contains(producer))
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
        ExecutionPlan::Guarded { .. }
        | ExecutionPlan::EndArrival { .. }
        | ExecutionPlan::Yield { .. } => {}
    }
}
