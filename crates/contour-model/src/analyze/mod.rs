//! Validates path-dependent flow invariants and builds an execution plan.

use std::collections::HashSet;

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Branch, Flow, Merge, Plan};

mod action;
mod choice;
mod merge;
mod question;

/// Walks every path through the flow exactly once, proving the invariants that
/// need path state and recording what model consumers can rely on.
pub(crate) fn flow(flow: &Flow) -> Result<Plan> {
    let mut analysis = Analysis {
        flow,
        visited: HashSet::new(),
    };
    let walked = analysis.walk(PathState {
        available: flow.sources.iter().cloned().collect(),
        executed: HashSet::new(),
        last: None,
    })?;
    if let Exit::Merge { index, .. } = walked.exit {
        return Err(Error::new(
            flow.blocks[index].span,
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

    Ok(walked.plan)
}

struct Analysis<'a> {
    flow: &'a Flow,
    visited: HashSet<usize>,
}

#[derive(Clone)]
struct PathState {
    available: HashSet<Ident>,
    executed: HashSet<usize>,
    last: Option<usize>,
}

struct Walked {
    plan: Plan,
    exit: Exit,
}

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

    /// Reports whether the path exits through a terminal action.
    fn terminates(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

struct MergePlan {
    index: usize,
    state: PathState,
}

impl Analysis<'_> {
    /// Analyzes the next ready block from one path state.
    fn walk(&mut self, state: PathState) -> Result<Walked> {
        let blocks = &self.flow.blocks;
        let ready = blocks
            .iter()
            .enumerate()
            .filter(|(index, block)| {
                !state.executed.contains(index)
                    && match block.kind {
                        BlockKind::Action | BlockKind::Question | BlockKind::Choice => {
                            regular_ready(block, &state)
                        }
                        BlockKind::Merge => merge::ready(block, &state),
                    }
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if ready.len() > 1 {
            return Err(Error::new(
                blocks[ready[1]].span,
                "multiple Contour blocks are ready at once; add an explicit dependency",
            ));
        }
        let Some(index) = ready.first().copied() else {
            return self.finish(&state);
        };

        match blocks[index].kind {
            BlockKind::Action => action::walk(self, index, state),
            BlockKind::Question => question::walk(self, index, state),
            BlockKind::Choice => choice::walk(self, index, state),
            BlockKind::Merge => merge::walk(self, index, state),
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

    /// Records how each branch leaves the branch point and, when they converge,
    /// walks the single continuation shared beyond the merge.
    fn branches(&mut self, walked: Vec<Walked>) -> Result<(Vec<Branch>, Option<Merge>, Exit)> {
        let Some(merge) = merge::plan(self, &walked)? else {
            // Without a merge every branch ends the flow and carries its own value.
            let states = walked
                .iter()
                .map(|path| path.exit.state())
                .collect::<Vec<_>>();
            let exit = Exit::Terminal(combine_states(&states));
            let branches = walked
                .into_iter()
                .map(|path| Branch {
                    plan: Box::new(path.plan),
                    early_return: false,
                })
                .collect();

            return Ok((branches, None, exit));
        };

        let branches = walked
            .into_iter()
            .map(|path| Branch {
                early_return: path.exit.terminates(),
                plan: Box::new(path.plan),
            })
            .collect();
        let index = merge.index;
        let continuation = self.walk(merge.state)?;

        Ok((
            branches,
            Some(Merge {
                index,
                next: Box::new(continuation.plan),
            }),
            continuation.exit,
        ))
    }

    /// Turns a completed terminal path into its final plan.
    fn finish(&self, state: &PathState) -> Result<Walked> {
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

        Ok(Walked {
            plan: Plan::Terminal {
                output: block.outputs[0].clone(),
            },
            exit: Exit::Terminal(state.clone()),
        })
    }
}

/// Reports whether all inputs of a computational block are available.
fn regular_ready(block: &Block, state: &PathState) -> bool {
    block
        .inputs
        .iter()
        .all(|input| state.available.contains(&input.ident))
}

/// Intersects available wires and unions executed blocks across paths.
fn combine_states(states: &[&PathState]) -> PathState {
    let mut combined = (*states
        .first()
        .expect("a question or choice has at least two paths"))
    .clone();
    for state in &states[1..] {
        combined
            .available
            .retain(|wire| state.available.contains(wire));
        combined.executed.extend(state.executed.iter().copied());
    }
    combined
}
