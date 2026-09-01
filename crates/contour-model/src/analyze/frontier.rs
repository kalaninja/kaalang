//! Resumable execution-plan frontiers and their public-plan conversion.

use proc_macro2::Ident;
use syn::Result;

use super::{Analysis, Exit, PathState, choice};
use crate::model::{BlockKind, Branch, Convergence, Merge, Plan};

pub(super) struct WorkPlan {
    pub(super) kind: WorkKind,
    /// Present only when this complete subtree exits the flow or reaches an
    /// authored merge. Yield leaves close into an ancestor convergence instead.
    pub(super) exit: Option<Exit>,
}

pub(super) enum WorkKind {
    Open {
        id: usize,
        state: PathState,
    },
    Action {
        index: usize,
        next: Box<WorkPlan>,
    },
    Question {
        id: usize,
        index: usize,
        branches: Vec<WorkPlan>,
        join: WorkJoin,
    },
    Choice {
        id: usize,
        index: usize,
        branches: Vec<WorkPlan>,
        join: WorkJoin,
    },
    Terminal {
        output: Ident,
    },
    Arrival {
        input: Ident,
        merge: usize,
    },
    Yield {
        wires: Vec<Ident>,
    },
}

pub(super) enum WorkJoin {
    None,
    Merge {
        index: usize,
        next: Box<WorkPlan>,
    },
    Convergence {
        wires: Vec<Ident>,
        next: Box<WorkPlan>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Terminal,
    Arrival,
    Yield,
}

pub(super) struct Candidate {
    pub(super) branch: usize,
    pub(super) frontiers: Vec<usize>,
    pub(super) wires: Vec<Ident>,
}

impl WorkPlan {
    pub(super) fn collect_frontiers<'a>(&'a self, frontiers: &mut Vec<(usize, &'a PathState)>) {
        match &self.kind {
            WorkKind::Open { id, state } => frontiers.push((*id, state)),
            WorkKind::Action { next, .. } => next.collect_frontiers(frontiers),
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::None => {
                        for branch in branches {
                            branch.collect_frontiers(frontiers);
                        }
                    }
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.collect_frontiers(frontiers);
                    }
                }
            }
            WorkKind::Terminal { .. } | WorkKind::Arrival { .. } | WorkKind::Yield { .. } => {}
        }
    }

    pub(super) fn frontier(&self, target: usize) -> Option<&PathState> {
        match &self.kind {
            WorkKind::Open { id, state } if *id == target => Some(state),
            WorkKind::Action { next, .. } => next.frontier(target),
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::None => branches.iter().find_map(|branch| branch.frontier(target)),
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.frontier(target)
                    }
                }
            }
            _ => None,
        }
    }

    pub(super) fn replace_frontier(&mut self, target: usize, replacement: WorkPlan) -> bool {
        match &mut self.kind {
            WorkKind::Open { id, .. } if *id == target => {
                *self = replacement;
                true
            }
            WorkKind::Action { next, .. } => next.replace_frontier(target, replacement),
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::None => {
                        for branch in branches {
                            if branch.frontier(target).is_some() {
                                return branch.replace_frontier(target, replacement);
                            }
                        }
                        false
                    }
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.replace_frontier(target, replacement)
                    }
                }
            }
            _ => false,
        }
    }

    pub(super) fn convergence_candidate(
        &self,
        analysis: &Analysis<'_>,
    ) -> Result<Option<Candidate>> {
        match &self.kind {
            WorkKind::Action { next, .. } => next.convergence_candidate(analysis),
            WorkKind::Question {
                id, branches, join, ..
            } => branch_candidate(analysis, *id, branches, join, None),
            WorkKind::Choice {
                id,
                index,
                branches,
                join,
            } => branch_candidate(analysis, *id, branches, join, Some(*index)),
            WorkKind::Open { .. }
            | WorkKind::Terminal { .. }
            | WorkKind::Arrival { .. }
            | WorkKind::Yield { .. } => Ok(None),
        }
    }

    pub(super) fn install_convergence(&mut self, target: usize, convergence: WorkJoin) -> bool {
        match &mut self.kind {
            WorkKind::Action { next, .. } => next.install_convergence(target, convergence),
            WorkKind::Question { id, join, .. } | WorkKind::Choice { id, join, .. }
                if *id == target =>
            {
                debug_assert!(matches!(join, WorkJoin::None));
                *join = convergence;
                true
            }
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::None => {
                        for branch in branches {
                            if branch.contains_branch(target) {
                                return branch.install_convergence(target, convergence);
                            }
                        }
                        false
                    }
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.install_convergence(target, convergence)
                    }
                }
            }
            _ => false,
        }
    }

    fn contains_branch(&self, target: usize) -> bool {
        match &self.kind {
            WorkKind::Action { next, .. } => next.contains_branch(target),
            WorkKind::Question {
                id, branches, join, ..
            }
            | WorkKind::Choice {
                id, branches, join, ..
            } => {
                *id == target
                    || match join {
                        WorkJoin::None => {
                            branches.iter().any(|branch| branch.contains_branch(target))
                        }
                        WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                            next.contains_branch(target)
                        }
                    }
            }
            _ => false,
        }
    }

    pub(super) fn collect_terminal_states(
        &self,
        target: usize,
        visit: &mut impl FnMut(&PathState),
    ) -> bool {
        match &self.kind {
            WorkKind::Action { next, .. } => next.collect_terminal_states(target, visit),
            WorkKind::Question {
                id, branches, join, ..
            }
            | WorkKind::Choice {
                id, branches, join, ..
            } => {
                if *id == target {
                    for branch in branches {
                        branch.visit_terminal_states(visit);
                    }
                    true
                } else {
                    match join {
                        WorkJoin::None => branches
                            .iter()
                            .any(|branch| branch.collect_terminal_states(target, visit)),
                        WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                            next.collect_terminal_states(target, visit)
                        }
                    }
                }
            }
            _ => false,
        }
    }

    fn visit_terminal_states(&self, visit: &mut impl FnMut(&PathState)) {
        if let Some(Exit::Terminal(state)) = &self.exit {
            visit(state);
            return;
        }
        match &self.kind {
            WorkKind::Action { next, .. } => next.visit_terminal_states(visit),
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::None => {
                        for branch in branches {
                            branch.visit_terminal_states(visit);
                        }
                    }
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.visit_terminal_states(visit);
                    }
                }
            }
            _ => {}
        }
    }

    fn outcome(&self) -> Outcome {
        match &self.kind {
            WorkKind::Action { next, .. } => next.outcome(),
            WorkKind::Question { branches, join, .. } | WorkKind::Choice { branches, join, .. } => {
                match join {
                    WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } => {
                        next.outcome()
                    }
                    WorkJoin::None => {
                        let outcomes = branches.iter().map(WorkPlan::outcome).collect::<Vec<_>>();
                        if outcomes.iter().all(|outcome| *outcome == Outcome::Terminal) {
                            Outcome::Terminal
                        } else if outcomes.contains(&Outcome::Yield) {
                            Outcome::Yield
                        } else {
                            Outcome::Arrival
                        }
                    }
                }
            }
            WorkKind::Terminal { .. } => Outcome::Terminal,
            WorkKind::Arrival { .. } => Outcome::Arrival,
            WorkKind::Yield { .. } => Outcome::Yield,
            WorkKind::Open { .. } => panic!("an open frontier has no completed outcome"),
        }
    }

    pub(super) fn into_plan(self) -> Plan {
        match self.kind {
            WorkKind::Open { .. } => panic!("the completed semantic plan has no open frontiers"),
            WorkKind::Action { index, next } => Plan::Action {
                index,
                next: Box::new(next.into_plan()),
            },
            WorkKind::Question {
                index,
                branches,
                join,
                ..
            } => {
                let (merge, convergence) = join.into_public();
                let branches = public_branches(branches, merge.is_some() || convergence.is_some());
                let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
                    unreachable!("a question declares exactly two outputs")
                };
                Plan::Question {
                    index,
                    branches,
                    merge,
                    convergence,
                }
            }
            WorkKind::Choice {
                index,
                branches,
                join,
                ..
            } => {
                let (merge, convergence) = join.into_public();
                Plan::Choice {
                    index,
                    branches: public_branches(branches, merge.is_some() || convergence.is_some()),
                    merge,
                    convergence,
                }
            }
            WorkKind::Terminal { output } => Plan::Terminal { output },
            WorkKind::Arrival { input, merge } => Plan::Arrival { input, merge },
            WorkKind::Yield { wires } => Plan::Yield { wires },
        }
    }
}

/// Reports the convergence one branch point is ready for: every frontier still
/// open below it waits on the same authored block. Branches whose paths already
/// ended contribute no frontier and take no part in it.
fn branch_candidate(
    analysis: &Analysis<'_>,
    id: usize,
    branches: &[WorkPlan],
    join: &WorkJoin,
    choice: Option<usize>,
) -> Result<Option<Candidate>> {
    if let WorkJoin::Merge { next, .. } | WorkJoin::Convergence { next, .. } = join {
        return next.convergence_candidate(analysis);
    }

    let mut frontiers = Vec::new();
    let mut continuing = Vec::with_capacity(branches.len());
    for branch in branches {
        let opened = frontiers.len();
        branch.collect_frontiers(&mut frontiers);
        continuing.push(frontiers.len() > opened);
    }
    if frontiers.len() > 1 {
        let ready = frontiers
            .iter()
            .map(|(_, state)| analysis.ready(state))
            .collect::<Result<Vec<_>>>()?;
        if let Some(Some(shared)) = ready.first()
            && ready.iter().all(|candidate| candidate == &Some(*shared))
            && analysis.flow.blocks[*shared].kind != BlockKind::Merge
        {
            if let Some(index) = choice {
                choice::adjacent_branches(&continuing, &analysis.flow.blocks[index].outputs)?;
            }
            let wires = analysis.flow.blocks[*shared]
                .inputs
                .iter()
                .filter(|input| {
                    let first = frontiers[0].1.available[&input.ident].origin;
                    frontiers[1..]
                        .iter()
                        .any(|(_, state)| state.available[&input.ident].origin != first)
                })
                .map(|input| input.ident.clone())
                .collect();
            return Ok(Some(Candidate {
                branch: id,
                frontiers: frontiers.iter().map(|(id, _)| *id).collect(),
                wires,
            }));
        }
    }

    for branch in branches {
        if let Some(candidate) = branch.convergence_candidate(analysis)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

impl WorkJoin {
    fn into_public(self) -> (Option<Merge>, Option<Convergence>) {
        match self {
            Self::None => (None, None),
            Self::Merge { index, next } => (
                Some(Merge {
                    index,
                    next: Box::new(next.into_plan()),
                }),
                None,
            ),
            Self::Convergence { wires, next } => (
                None,
                Some(Convergence {
                    wires,
                    next: Box::new(next.into_plan()),
                }),
            ),
        }
    }
}

fn public_branches(branches: Vec<WorkPlan>, joined: bool) -> Vec<Branch> {
    let outcomes = branches.iter().map(WorkPlan::outcome).collect::<Vec<_>>();
    let continuing = joined || outcomes.iter().any(|outcome| *outcome != Outcome::Terminal);
    branches
        .into_iter()
        .zip(outcomes)
        .map(|(plan, outcome)| Branch {
            plan: Box::new(plan.into_plan()),
            early_return: continuing && outcome == Outcome::Terminal,
        })
        .collect()
}
