//! Validates path-dependent flow invariants and builds an execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Flow, Plan};

mod action;
mod choice;
mod end;
mod frontier;
mod question;

use frontier::{Candidate, WorkJoin, WorkKind, WorkPlan};

/// Walks every path through the flow exactly once, proving the invariants that
/// need path state and recording what model consumers can rely on.
pub(crate) fn flow(flow: &Flow) -> Result<Plan> {
    let end = flow.blocks.len() - 1;
    debug_assert_eq!(flow.blocks[end].kind, BlockKind::End);
    let sources = flow
        .sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            (
                source.clone(),
                AvailableWire {
                    ident: source.clone(),
                    origin: Producer::Source(index),
                },
            )
        })
        .collect();
    let produced = flow.sources.iter().cloned().collect();
    let unconsumed = flow
        .sources
        .iter()
        .map(|source| (source.clone(), source.clone()))
        .collect();
    let mut analysis = Analysis {
        flow,
        visited: HashSet::new(),
        next_frontier: 0,
        next_branch: 0,
    };
    let mut work = analysis.open(PathState {
        available: sources,
        produced,
        unconsumed,
        executed: HashSet::new(),
    });
    analysis.run(&mut work)?;

    if let Some(unreachable) = flow
        .blocks
        .iter()
        .enumerate()
        .find(|(index, _)| !analysis.visited.contains(index))
        .map(|(_, block)| block)
    {
        return Err(Error::new(
            unreachable.span,
            "this Contour block is unreachable",
        ));
    }

    Ok(Plan::End {
        index: end,
        body: Box::new(work.into_plan()),
    })
}

struct Analysis<'a> {
    flow: &'a Flow,
    visited: HashSet<usize>,
    next_frontier: usize,
    next_branch: usize,
}

#[derive(Clone)]
struct AvailableWire {
    /// The producer occurrence whose span should be used when this value is
    /// yielded from a branch expression.
    ident: Ident,
    origin: Producer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Producer {
    Source(usize),
    Block { index: usize, output: usize },
    Convergence { branch: usize, wire: usize },
}

#[derive(Clone)]
struct PathState {
    available: HashMap<Ident, AvailableWire>,
    produced: HashSet<Ident>,
    unconsumed: HashMap<Ident, Ident>,
    executed: HashSet<usize>,
}

impl PathState {
    /// Makes one output available unless this path already produced its name.
    fn produce(&mut self, output: &Ident, origin: Producer) -> Result<()> {
        if !self.produced.insert(output.clone()) {
            return Err(Error::new(
                output.span(),
                "a Contour wire must not be produced more than once on the same path",
            ));
        }
        self.available.insert(
            output.clone(),
            AvailableWire {
                ident: output.clone(),
                origin,
            },
        );
        self.unconsumed.insert(output.clone(), output.clone());
        Ok(())
    }
}

impl Analysis<'_> {
    fn run(&mut self, work: &mut WorkPlan) -> Result<()> {
        loop {
            self.settle(work)?;
            if work.exit.is_some() {
                return Ok(());
            }

            if let Some(candidate) = work.convergence_candidate(self)? {
                self.converge(work, candidate);
                continue;
            }

            let mut frontiers = Vec::new();
            work.collect_frontiers(&mut frontiers);
            let ready = frontiers
                .into_iter()
                .map(|(id, state)| Ok((id, self.ready(state)?)))
                .collect::<Result<Vec<_>>>()?;
            let id = choose_frontier(&ready).expect("an unfinished plan has an open frontier");
            self.advance(work, id)?;
        }
    }

    fn open(&mut self, state: PathState) -> WorkPlan {
        let id = self.next_frontier;
        self.next_frontier += 1;
        WorkPlan {
            kind: WorkKind::Open { id, state },
            exit: None,
        }
    }

    fn branch_id(&mut self) -> usize {
        let id = self.next_branch;
        self.next_branch += 1;
        id
    }

    /// Reports the one block ready at a frontier, or `None` when the path has
    /// run out of blocks and must be finished.
    fn ready(&self, state: &PathState) -> Result<Option<usize>> {
        let ready = self
            .flow
            .blocks
            .iter()
            .enumerate()
            .filter(|(index, block)| {
                !state.executed.contains(index)
                    && match block.kind {
                        BlockKind::Action | BlockKind::Question | BlockKind::Choice => {
                            regular_ready(block, state)
                        }
                        BlockKind::End => false,
                    }
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if ready.len() > 1 {
            return Err(Error::new(
                self.flow.blocks[ready[1]].span,
                "multiple Contour blocks are ready at once; add an explicit dependency",
            ));
        }
        Ok(ready.first().copied())
    }

    fn advance(&mut self, work: &mut WorkPlan, frontier: usize) -> Result<()> {
        let state = work
            .frontier(frontier)
            .expect("the selected frontier belongs to the work plan")
            .clone();
        let replacement = match self.ready(&state)? {
            Some(index) => self.enter_block(index, state)?,
            None => self.finish(state)?,
        };
        work.replace_frontier(frontier, replacement);
        Ok(())
    }

    fn enter_block(&mut self, index: usize, state: PathState) -> Result<WorkPlan> {
        match self.flow.blocks[index].kind {
            BlockKind::Action => action::enter(self, index, state),
            BlockKind::Question => question::enter(self, index, state),
            BlockKind::Choice => choice::enter(self, index, state),
            BlockKind::End => unreachable!("End is entered only after a path is exhausted"),
        }
    }

    /// Enters a computational block and consumes its bare inputs.
    fn enter(&mut self, index: usize, mut state: PathState) -> PathState {
        self.visited.insert(index);
        state.executed.insert(index);
        for input in &self.flow.blocks[index].inputs {
            state.unconsumed.remove(&input.ident);
            if !input.borrowed {
                state.available.remove(&input.ident);
            }
        }
        state
    }

    fn finish(&mut self, state: PathState) -> Result<WorkPlan> {
        end::arrive(self, self.flow.blocks.len() - 1, state)
    }

    /// Propagates completed paths through shared continuations.
    fn settle(&mut self, plan: &mut WorkPlan) -> Result<()> {
        let exit = match &mut plan.kind {
            WorkKind::Open { .. } | WorkKind::EndArrival { .. } | WorkKind::Yield { .. } => None,
            WorkKind::Action { next, .. } => {
                self.settle(next)?;
                next.exit.clone()
            }
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                self.settle_branches(branches, join)?
            }
        };
        if plan.exit.is_none() {
            plan.exit = exit;
        }
        Ok(())
    }

    fn settle_branches(
        &mut self,
        branches: &mut [WorkPlan],
        join: &mut WorkJoin,
    ) -> Result<Option<PathState>> {
        match join {
            WorkJoin::Convergence { next, .. } => {
                self.settle(next)?;
                return Ok(next.exit.clone());
            }
            WorkJoin::None => {}
        }

        for branch in branches.iter_mut() {
            self.settle(branch)?;
        }
        let mut states = Vec::with_capacity(branches.len());
        for branch in branches.iter() {
            let Some(state) = &branch.exit else {
                return Ok(None);
            };
            states.push(state);
        }
        Ok(Some(combine_states(&states)))
    }

    fn converge(&mut self, work: &mut WorkPlan, candidate: Candidate) {
        let states = candidate
            .frontiers
            .iter()
            .map(|id| {
                work.frontier(*id)
                    .expect("a convergence frontier belongs to its branch")
                    .clone()
            })
            .collect::<Vec<_>>();
        let mut combined = combine_states(&states.iter().collect::<Vec<_>>());
        work.collect_end_states(candidate.branch, &mut |state| {
            combined.executed.extend(state.executed.iter().copied());
        });

        for (wire, ident) in candidate.wires.iter().enumerate() {
            combined.available.insert(
                ident.clone(),
                AvailableWire {
                    ident: ident.clone(),
                    origin: Producer::Convergence {
                        branch: candidate.branch,
                        wire,
                    },
                },
            );
        }

        let yielded = candidate
            .frontiers
            .iter()
            .map(|id| {
                let state = work
                    .frontier(*id)
                    .expect("a convergence frontier belongs to its branch");
                let wires = candidate
                    .wires
                    .iter()
                    .map(|wire| state.available[wire].ident.clone())
                    .collect::<Vec<_>>();
                (*id, wires)
            })
            .collect::<Vec<_>>();
        for (id, wires) in yielded {
            work.replace_frontier(
                id,
                WorkPlan {
                    kind: WorkKind::Yield { wires },
                    exit: None,
                },
            );
        }

        let next = self.open(combined);
        work.install_convergence(
            candidate.branch,
            WorkJoin::Convergence {
                wires: candidate.wires,
                next: Box::new(next),
            },
        );
    }
}

/// Reports whether all inputs of a computational block are available.
fn regular_ready(block: &Block, state: &PathState) -> bool {
    block
        .inputs
        .iter()
        .all(|input| state.available.contains_key(&input.ident))
}

/// Selects a frontier that is not waiting alongside an equivalent occurrence
/// of a possible shared consumer. This lets deeper sibling paths catch up.
fn choose_frontier(ready: &[(usize, Option<usize>)]) -> Option<usize> {
    if let Some((id, _)) = ready.iter().find(|(_, block)| block.is_none()) {
        return Some(*id);
    }
    let mut counts = HashMap::new();
    for (_, block) in ready {
        *counts.entry(*block).or_insert(0) += 1;
    }
    ready
        .iter()
        .filter(|(_, block)| counts[block] == 1)
        .min_by_key(|(_, block)| *block)
        .or_else(|| ready.iter().min_by_key(|(_, block)| *block))
        .map(|(id, _)| *id)
}

/// Intersects same-origin available wires and unions path history.
fn combine_states(states: &[&PathState]) -> PathState {
    let mut combined = (*states
        .first()
        .expect("a question or choice has at least one continuing path"))
    .clone();
    for state in &states[1..] {
        combined.available.retain(|name, available| {
            state
                .available
                .get(name)
                .is_some_and(|other| other.origin == available.origin)
        });
        combined.produced.extend(state.produced.iter().cloned());
        combined.unconsumed.extend(
            state
                .unconsumed
                .iter()
                .map(|(name, span)| (name.clone(), span.clone())),
        );
        combined.executed.extend(state.executed.iter().copied());
    }
    combined
}
