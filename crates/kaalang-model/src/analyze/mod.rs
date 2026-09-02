//! Validates path-dependent flow invariants and builds an execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Flow, Plan};

mod action;
mod choice;
mod end;
mod frontier;

use frontier::{Candidate, WorkJoin, WorkKind, WorkPlan};

/// Walks every path through the flow exactly once, proving the invariants that
/// need path state and recording what model consumers can rely on.
pub(crate) fn flow(flow: &Flow) -> Result<Plan> {
    let end = flow.blocks.len() - 1;
    debug_assert_eq!(flow.blocks[end].kind, BlockKind::End);
    let mut analysis = Analysis {
        flow,
        visited: HashSet::new(),
        next_id: 0,
    };
    let mut work = analysis.open(PathState {
        available: flow
            .sources
            .iter()
            .map(|source| (source.clone(), Producer::Source))
            .collect(),
        produced: flow.sources.iter().cloned().collect(),
        unconsumed: flow.sources.iter().cloned().collect(),
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
            "this kaalang block is unreachable",
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
    /// Frontiers and branch points draw ids from one counter; each only needs
    /// to be unique.
    next_id: usize,
}

/// The occurrence that made a wire available on a path. Origins are compared
/// only for one wire name at a time, and sources, one block's outputs, and one
/// convergence's wires are each unique by name, so the owner identifies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Producer {
    Source,
    Block(usize),
    Convergence(usize),
}

#[derive(Clone)]
struct PathState {
    /// Every wire a block on this path may still consume. The key is the
    /// producer occurrence whose span a branch yield reports against.
    available: HashMap<Ident, Producer>,
    produced: HashSet<Ident>,
    unconsumed: HashSet<Ident>,
    executed: HashSet<usize>,
}

impl PathState {
    /// Makes one output available unless this path already produced its name.
    fn produce(&mut self, output: &Ident, origin: Producer) -> Result<()> {
        if !self.produced.insert(output.clone()) {
            return Err(Error::new(
                output.span(),
                "a kaalang wire must not be produced more than once on the same path",
            ));
        }
        self.available.insert(output.clone(), origin);
        self.unconsumed.insert(output.clone());
        Ok(())
    }
}

impl Analysis<'_> {
    fn run(&mut self, work: &mut WorkPlan) -> Result<()> {
        loop {
            work.settle();
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

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn open(&mut self, state: PathState) -> WorkPlan {
        WorkPlan {
            kind: WorkKind::Open {
                id: self.next_id(),
                state,
            },
            exit: None,
        }
    }

    /// Reports the one block ready at a frontier, or `None` when the path has
    /// run out of blocks and must be finished. End counts toward readiness
    /// even after a converged sibling path executed it.
    fn ready(&self, state: &PathState) -> Result<Option<usize>> {
        let end = self.flow.blocks.len() - 1;
        let ready = self
            .flow
            .blocks
            .iter()
            .enumerate()
            .filter(|(index, block)| {
                (*index == end || !state.executed.contains(index)) && inputs_available(block, state)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        match ready.as_slice() {
            [] => Ok(None),
            [first] => Ok((*first != end).then_some(*first)),
            [_, conflict, ..] => Err(Error::new(
                self.flow.blocks[*conflict].span,
                "multiple kaalang blocks are ready at once; add an explicit dependency",
            )),
        }
    }

    fn advance(&mut self, work: &mut WorkPlan, frontier: usize) -> Result<()> {
        let state = work
            .frontier(frontier)
            .expect("the selected frontier belongs to the work plan")
            .clone();
        let end = self.flow.blocks.len() - 1;
        let replacement = match self.ready(&state)? {
            Some(index) => self.enter_block(index, state)?,
            None => end::arrive(self, end, state)?,
        };
        work.replace_frontier(frontier, replacement);
        Ok(())
    }

    fn enter_block(&mut self, index: usize, state: PathState) -> Result<WorkPlan> {
        match self.flow.blocks[index].kind {
            BlockKind::Action => action::enter(self, index, state),
            BlockKind::Question | BlockKind::Choice => self.branch_point(index, state),
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

    /// Opens one frontier per output of a question or choice, each producing
    /// only the output that selects it.
    fn branch_point(&mut self, index: usize, state: PathState) -> Result<WorkPlan> {
        let next = self.enter(index, state);
        let outputs = &self.flow.blocks[index].outputs;
        let mut branches = Vec::with_capacity(outputs.len());
        for output in outputs {
            let mut branch = next.clone();
            branch.produce(output, Producer::Block(index))?;
            branches.push(self.open(branch));
        }
        Ok(WorkPlan {
            kind: WorkKind::Branching {
                id: self.next_id(),
                index,
                kind: self.flow.blocks[index].kind,
                branches,
                join: None,
            },
            exit: None,
        })
    }

    fn converge(&mut self, work: &mut WorkPlan, candidate: Candidate) {
        let states = candidate
            .frontiers
            .iter()
            .map(|id| {
                work.frontier(*id)
                    .expect("a convergence frontier belongs to its branch")
            })
            .collect::<Vec<_>>();
        let mut combined = combine_states(&states);
        work.collect_end_states(candidate.branch, &mut |state| {
            combined.executed.extend(state.executed.iter().copied());
        });
        for (wire, ident) in candidate.wires.iter().enumerate() {
            combined
                .available
                .insert(ident.clone(), Producer::Convergence(wire));
        }

        let yielded = states
            .iter()
            .map(|state| {
                candidate
                    .wires
                    .iter()
                    .map(|wire| {
                        let (ident, _) = state
                            .available
                            .get_key_value(wire)
                            .expect("a convergence wire is available on every converging frontier");
                        ident.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for (id, wires) in candidate.frontiers.iter().zip(yielded) {
            work.replace_frontier(
                *id,
                WorkPlan {
                    kind: WorkKind::Yield { wires },
                    exit: None,
                },
            );
        }

        let next = self.open(combined);
        work.install_convergence(
            candidate.branch,
            WorkJoin {
                wires: candidate.wires,
                next: Box::new(next),
            },
        );
    }
}

/// Reports whether all inputs of a block are available.
fn inputs_available(block: &Block, state: &PathState) -> bool {
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
        combined
            .available
            .retain(|name, origin| state.available.get(name) == Some(&*origin));
        combined.produced.extend(state.produced.iter().cloned());
        combined.unconsumed.extend(state.unconsumed.iter().cloned());
        combined.executed.extend(state.executed.iter().copied());
    }
    combined
}
