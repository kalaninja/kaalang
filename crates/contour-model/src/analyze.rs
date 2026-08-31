//! Validates path-dependent flow invariants and builds an execution plan.

use std::collections::HashSet;

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, Branch, Flow, Merge, Plan};

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
                    && if block.kind == BlockKind::Merge {
                        block
                            .inputs
                            .iter()
                            .any(|input| state.available.contains(&input.ident))
                    } else {
                        block
                            .inputs
                            .iter()
                            .all(|input| state.available.contains(&input.ident))
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

        let block = &blocks[index];
        if block.kind == BlockKind::Merge {
            let available = block
                .inputs
                .iter()
                .filter(|input| state.available.contains(&input.ident))
                .collect::<Vec<_>>();
            debug_assert!(!available.is_empty(), "a ready merge has an input");
            if available.len() != 1 {
                return Err(Error::new(
                    available[1].ident.span(),
                    "exactly one Contour merge input must be available on each path",
                ));
            }

            let input = available[0].ident.clone();
            let mut state = state;
            state.available.remove(&input);
            return Ok(Walked {
                plan: Plan::Arrival {
                    input: input.clone(),
                    merge: index,
                },
                exit: Exit::Merge {
                    index,
                    input,
                    state,
                },
            });
        }

        self.visited.insert(index);
        let mut next = state;
        next.executed.insert(index);
        next.last = Some(index);
        for input in &block.inputs {
            if !input.borrowed {
                next.available.remove(&input.ident);
            }
        }

        match block.kind {
            BlockKind::Action => self.action(index, next),
            BlockKind::Question => self.question(index, next),
            BlockKind::Choice => self.choice(index, &next),
            BlockKind::Merge => unreachable!("merge blocks return before ordinary analysis"),
        }
    }

    /// Adds an action's outputs and analyzes its continuation.
    fn action(&mut self, index: usize, mut next: PathState) -> Result<Walked> {
        for output in &self.flow.blocks[index].outputs {
            next.available.insert(output.clone());
        }
        let continuation = self.walk(next)?;

        Ok(Walked {
            plan: Plan::Action {
                index,
                next: Box::new(continuation.plan),
            },
            exit: continuation.exit,
        })
    }

    /// Analyzes both positional branches of a question.
    fn question(&mut self, index: usize, next: PathState) -> Result<Walked> {
        let outputs = &self.flow.blocks[index].outputs;
        let mut yes_state = next.clone();
        yes_state.available.insert(outputs[0].clone());
        let mut no_state = next;
        no_state.available.insert(outputs[1].clone());
        let walked = vec![self.walk(yes_state)?, self.walk(no_state)?];
        let merge = self.merge_plan(&walked)?;
        let (branches, merge, exit) = self.branches(walked, merge)?;
        let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
            unreachable!("a question declares exactly two outputs")
        };

        Ok(Walked {
            plan: Plan::Question {
                index,
                branches,
                merge,
            },
            exit,
        })
    }

    /// Analyzes every ordered case branch of a choice.
    fn choice(&mut self, index: usize, next: &PathState) -> Result<Walked> {
        let outputs = &self.flow.blocks[index].outputs;
        let mut walked = Vec::with_capacity(outputs.len());
        for output in outputs {
            let mut branch_state = next.clone();
            branch_state.available.insert(output.clone());
            walked.push(self.walk(branch_state)?);
        }
        adjacent_branches(&walked, outputs)?;
        let merge = self.merge_plan(&walked)?;
        let (branches, merge, exit) = self.branches(walked, merge)?;

        Ok(Walked {
            plan: Plan::Choice {
                index,
                branches,
                merge,
            },
            exit,
        })
    }

    /// Records how each branch leaves the branch point and, when they converge,
    /// walks the single continuation shared beyond the merge.
    fn branches(
        &mut self,
        walked: Vec<Walked>,
        merge: Option<MergePlan>,
    ) -> Result<(Vec<Branch>, Option<Merge>, Exit)> {
        let Some(merge) = merge else {
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

    /// Validates branch convergence and computes the post-merge state.
    fn merge_plan(&mut self, paths: &[Walked]) -> Result<Option<MergePlan>> {
        let blocks = &self.flow.blocks;
        let arrivals = paths
            .iter()
            .filter_map(|path| match &path.exit {
                Exit::Merge {
                    index,
                    input,
                    state,
                } => Some((*index, input, state)),
                Exit::Terminal(_) => None,
            })
            .collect::<Vec<_>>();
        let Some((index, _, _)) = arrivals.first().copied() else {
            return Ok(None);
        };
        if let Some((other, _, _)) = arrivals.iter().find(|(other, _, _)| *other != index) {
            return Err(Error::new(
                blocks[*other].span,
                "continuing branches must reach the same Contour merge",
            ));
        }

        let merge = &blocks[index];
        let mut arrived = HashSet::new();
        for (_, input, _) in &arrivals {
            if !arrived.insert((*input).clone()) {
                return Err(Error::new(
                    input.span(),
                    "two branches reach the same Contour merge input",
                ));
            }
        }
        let declared = merge
            .inputs
            .iter()
            .map(|input| input.ident.clone())
            .collect::<HashSet<_>>();
        if arrived != declared {
            return Err(Error::new(
                merge.span,
                "every Contour merge input must be reached by exactly one branch",
            ));
        }

        let states = arrivals
            .iter()
            .map(|(_, _, state)| *state)
            .collect::<Vec<_>>();
        let mut state = combine_states(&states);
        for path in paths {
            state
                .executed
                .extend(path.exit.state().executed.iter().copied());
        }
        for input in &merge.inputs {
            state.available.remove(&input.ident);
        }
        state.available.insert(merge.outputs[0].clone());
        state.executed.insert(index);
        state.last = Some(index);
        self.visited.insert(index);

        Ok(Some(MergePlan { index, state }))
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

/// Rejects a case that ends the flow between two cases that continue, which no
/// skewer order can draw: the later continuing branch reaches its merge by
/// crossing the ending branch. A question cannot reach this, because a merge it
/// feeds is reached by both of its branches or by neither.
fn adjacent_branches(walked: &[Walked], outputs: &[Ident]) -> Result<()> {
    let continuing = |path: &Walked| !path.exit.terminates();
    let Some(first) = walked.iter().position(continuing) else {
        return Ok(());
    };
    let last = walked
        .iter()
        .rposition(continuing)
        .expect("a continuing branch was just found");
    let Some(offset) = walked[first..last]
        .iter()
        .position(|path| !continuing(path))
    else {
        return Ok(());
    };

    Err(Error::new(
        outputs[first + offset].span(),
        "a Contour case that ends the flow must not separate cases that continue to a merge",
    ))
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
