//! Validates path-dependent flow invariants and builds an execution plan.

use std::collections::{HashMap, HashSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Flow, Plan};

mod action;
mod choice;
mod frontier;
mod merge;
mod question;

use frontier::{Candidate, WorkJoin, WorkKind, WorkPlan};

/// Walks every path through the flow exactly once, proving the invariants that
/// need path state and recording what model consumers can rely on.
pub(crate) fn flow(flow: &Flow) -> Result<Plan> {
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
    let mut analysis = Analysis {
        flow,
        visited: HashSet::new(),
        next_frontier: 0,
        next_branch: 0,
    };
    let mut work = analysis.open(PathState {
        available: sources,
        produced,
        executed: HashSet::new(),
        last: None,
    });
    analysis.run(&mut work)?;

    if let Some(Exit::Merge { index, .. }) = &work.exit {
        return Err(Error::new(
            flow.blocks[*index].span,
            "a Contour merge must combine branches of a question or choice",
        ));
    }
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

    Ok(work.into_plan())
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
    executed: HashSet<usize>,
    last: Option<usize>,
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
        Ok(())
    }
}

#[derive(Clone)]
enum Exit {
    Terminal(PathState),
    Merge {
        index: usize,
        input: Ident,
        state: PathState,
    },
}

impl Exit {
    /// Returns the path state carried by either exit kind.
    fn state(&self) -> &PathState {
        match self {
            Self::Terminal(state) | Self::Merge { state, .. } => state,
        }
    }

    fn terminates(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

struct MergePlan {
    index: usize,
    state: PathState,
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
                        BlockKind::Merge => merge::ready(block, state),
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
            BlockKind::Merge => self.arrive(index, state),
        }
    }

    /// Enters a computational block and consumes its bare inputs.
    fn enter(&mut self, index: usize, mut state: PathState) -> PathState {
        self.visited.insert(index);
        state.executed.insert(index);
        state.last = Some(index);
        for input in &self.flow.blocks[index].inputs {
            if !input.borrowed {
                state.available.remove(&input.ident);
            }
        }
        state
    }

    fn arrive(&self, index: usize, mut state: PathState) -> Result<WorkPlan> {
        let block = &self.flow.blocks[index];
        let available = block
            .inputs
            .iter()
            .filter(|input| state.available.contains_key(&input.ident))
            .collect::<Vec<_>>();
        debug_assert!(!available.is_empty(), "a ready merge has an input");
        if available.len() != 1 {
            return Err(Error::new(
                available[1].ident.span(),
                "exactly one Contour merge input must be available on each path",
            ));
        }

        let input = available[0].ident.clone();
        state.available.remove(&input);
        Ok(WorkPlan {
            kind: WorkKind::Arrival {
                input: input.clone(),
                merge: index,
            },
            exit: Some(Exit::Merge {
                index,
                input,
                state,
            }),
        })
    }

    fn finish(&self, state: PathState) -> Result<WorkPlan> {
        let last = state
            .last
            .expect("the first block is always ready, so a finished path executed one");
        let block = &self.flow.blocks[last];
        if !block.terminal {
            return Err(Error::new(
                block.span,
                "this Contour path does not terminate in an action",
            ));
        }

        Ok(WorkPlan {
            kind: WorkKind::Terminal {
                output: block.outputs[0].clone(),
            },
            exit: Some(Exit::Terminal(state)),
        })
    }

    /// Propagates completed exits and opens legacy merge continuations.
    fn settle(&mut self, plan: &mut WorkPlan) -> Result<()> {
        let exit = match &mut plan.kind {
            WorkKind::Open { .. }
            | WorkKind::Terminal { .. }
            | WorkKind::Arrival { .. }
            | WorkKind::Yield { .. } => None,
            WorkKind::Action { next, .. } => {
                self.settle(next)?;
                next.exit.clone()
            }
            WorkKind::Question { branches, join, .. } => {
                self.settle_branches(branches, join, None)?
            }
            WorkKind::Choice {
                index,
                branches,
                join,
                ..
            } => self.settle_branches(branches, join, Some(*index))?,
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
        choice: Option<usize>,
    ) -> Result<Option<Exit>> {
        match join {
            WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                self.settle(next)?;
                return Ok(next.exit.clone());
            }
            WorkJoin::None => {}
        }

        for branch in branches.iter_mut() {
            self.settle(branch)?;
        }
        let Some(exits) = branches
            .iter()
            .map(|branch| branch.exit.clone())
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };

        let Some(merged) = merge::plan(self, &exits)? else {
            let states = exits.iter().map(Exit::state).collect::<Vec<_>>();
            return Ok(Some(Exit::Terminal(combine_states(&states))));
        };
        if let Some(index) = choice {
            let continuing = exits
                .iter()
                .map(|exit| !exit.terminates())
                .collect::<Vec<_>>();
            choice::adjacent_branches(&continuing, &self.flow.blocks[index].outputs)?;
        }
        *join = WorkJoin::Merge {
            index: merged.index,
            next: Box::new(self.open(merged.state)),
        };
        Ok(None)
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
        work.collect_terminal_states(candidate.branch, &mut |state| {
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
        combined.executed.extend(state.executed.iter().copied());
    }
    combined
}
