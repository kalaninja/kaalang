//! Resumable execution-plan frontiers and their public-plan conversion.

use std::slice;

use proc_macro2::Ident;
use syn::Result;

use super::{Analysis, PathState, choice, combine_states};
use crate::model::{BlockKind, Branch, Convergence, Plan};

pub(super) struct WorkPlan {
    pub(super) kind: WorkKind,
    /// Present when this complete subtree reaches End. Yield leaves close into
    /// an ancestor convergence instead.
    pub(super) exit: Option<PathState>,
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
    /// A question or choice; the block kind decides its public plan variant.
    Branching {
        id: usize,
        index: usize,
        kind: BlockKind,
        branches: Vec<WorkPlan>,
        join: Option<WorkJoin>,
    },
    EndArrival {
        inputs: Vec<Ident>,
    },
    Yield {
        wires: Vec<Ident>,
    },
}

/// The implicit convergence sibling branches yield into, and what follows it.
pub(super) struct WorkJoin {
    pub(super) wires: Vec<Ident>,
    pub(super) next: Box<WorkPlan>,
}

pub(super) struct Candidate {
    pub(super) branch: usize,
    pub(super) frontiers: Vec<usize>,
    pub(super) wires: Vec<Ident>,
}

impl WorkPlan {
    /// The subtrees that continue this plan: an action's continuation, a
    /// converged branch point's shared continuation, or its open branches.
    fn children(&self) -> impl Iterator<Item = &WorkPlan> {
        let children: &[WorkPlan] = match &self.kind {
            WorkKind::Action { next, .. } => slice::from_ref(next.as_ref()),
            WorkKind::Branching {
                join: Some(join), ..
            } => slice::from_ref(join.next.as_ref()),
            WorkKind::Branching { branches, .. } => branches,
            WorkKind::Open { .. } | WorkKind::EndArrival { .. } | WorkKind::Yield { .. } => &[],
        };
        children.iter()
    }

    fn children_mut(&mut self) -> impl Iterator<Item = &mut WorkPlan> {
        let children: &mut [WorkPlan] = match &mut self.kind {
            WorkKind::Action { next, .. } => slice::from_mut(next.as_mut()),
            WorkKind::Branching {
                join: Some(join), ..
            } => slice::from_mut(join.next.as_mut()),
            WorkKind::Branching { branches, .. } => branches,
            WorkKind::Open { .. } | WorkKind::EndArrival { .. } | WorkKind::Yield { .. } => &mut [],
        };
        children.iter_mut()
    }

    pub(super) fn collect_frontiers<'a>(&'a self, frontiers: &mut Vec<(usize, &'a PathState)>) {
        if let WorkKind::Open { id, state } = &self.kind {
            frontiers.push((*id, state));
        }
        for child in self.children() {
            child.collect_frontiers(frontiers);
        }
    }

    pub(super) fn frontier(&self, target: usize) -> Option<&PathState> {
        match &self.kind {
            WorkKind::Open { id, state } if *id == target => Some(state),
            _ => self.children().find_map(|child| child.frontier(target)),
        }
    }

    pub(super) fn replace_frontier(&mut self, target: usize, replacement: WorkPlan) {
        if matches!(&self.kind, WorkKind::Open { id, .. } if *id == target) {
            *self = replacement;
            return;
        }
        if let Some(child) = self
            .children_mut()
            .find(|child| child.frontier(target).is_some())
        {
            child.replace_frontier(target, replacement);
        }
    }

    pub(super) fn convergence_candidate(
        &self,
        analysis: &Analysis<'_>,
    ) -> Result<Option<Candidate>> {
        if let WorkKind::Branching {
            id,
            index,
            kind,
            branches,
            join: None,
        } = &self.kind
            && let Some(candidate) = branch_candidate(analysis, *id, *index, *kind, branches)?
        {
            return Ok(Some(candidate));
        }
        for child in self.children() {
            if let Some(candidate) = child.convergence_candidate(analysis)? {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    pub(super) fn install_convergence(&mut self, target: usize, convergence: WorkJoin) {
        if let WorkKind::Branching { id, join, .. } = &mut self.kind
            && *id == target
        {
            debug_assert!(join.is_none());
            *join = Some(convergence);
            return;
        }
        if let Some(child) = self
            .children_mut()
            .find(|child| child.contains_branch(target))
        {
            child.install_convergence(target, convergence);
        }
    }

    fn contains_branch(&self, target: usize) -> bool {
        matches!(&self.kind, WorkKind::Branching { id, .. } if *id == target)
            || self.children().any(|child| child.contains_branch(target))
    }

    /// Visits the exit state of every path below one branch point that ended.
    pub(super) fn collect_end_states(&self, target: usize, visit: &mut impl FnMut(&PathState)) {
        match &self.kind {
            WorkKind::Branching { id, branches, .. } if *id == target => {
                for branch in branches {
                    branch.visit_end_states(visit);
                }
            }
            _ => {
                for child in self.children() {
                    child.collect_end_states(target, visit);
                }
            }
        }
    }

    fn visit_end_states(&self, visit: &mut impl FnMut(&PathState)) {
        if let Some(state) = &self.exit {
            visit(state);
            return;
        }
        for child in self.children() {
            child.visit_end_states(visit);
        }
    }

    /// Propagates completed paths through shared continuations: a plan exits
    /// once every child that continues it does.
    pub(super) fn settle(&mut self) {
        for child in self.children_mut() {
            child.settle();
        }
        if self.exit.is_some() {
            return;
        }
        let exits = self
            .children()
            .map(|child| child.exit.as_ref())
            .collect::<Option<Vec<_>>>();
        if let Some(states) = exits.as_deref()
            && !states.is_empty()
        {
            self.exit = Some(combine_states(states));
        }
    }

    /// Reports whether this completed subtree yields into an ancestor
    /// convergence instead of reaching End.
    fn yields(&self) -> bool {
        match &self.kind {
            WorkKind::Yield { .. } => true,
            WorkKind::EndArrival { .. } => false,
            WorkKind::Open { .. } => panic!("an open frontier has no completed outcome"),
            WorkKind::Action { .. } | WorkKind::Branching { .. } => {
                self.children().any(WorkPlan::yields)
            }
        }
    }

    pub(super) fn into_plan(self) -> Plan {
        match self.kind {
            WorkKind::Open { .. } => panic!("the completed semantic plan has no open frontiers"),
            WorkKind::Action { index, next } => Plan::Action {
                index,
                next: Box::new(next.into_plan()),
            },
            WorkKind::Branching {
                index,
                kind,
                branches,
                join,
                ..
            } => {
                let convergence = join.map(|join| Convergence {
                    wires: join.wires,
                    next: Box::new(join.next.into_plan()),
                });
                let branches = public_branches(branches, convergence.is_some());
                match kind {
                    BlockKind::Question => {
                        let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
                            unreachable!("a question declares exactly two outputs")
                        };
                        Plan::Question {
                            index,
                            branches,
                            convergence,
                        }
                    }
                    BlockKind::Choice => Plan::Choice {
                        index,
                        branches,
                        convergence,
                    },
                    BlockKind::Action | BlockKind::End => {
                        unreachable!("only questions and choices branch")
                    }
                }
            }
            WorkKind::EndArrival { inputs } => Plan::EndArrival { inputs },
            WorkKind::Yield { wires } => Plan::Yield { wires },
        }
    }
}

/// Reports the convergence one unconverged branch point is ready for: every
/// frontier still open below it waits on the same authored block. Branches
/// whose paths already ended contribute no frontier and take no part in it.
fn branch_candidate(
    analysis: &Analysis<'_>,
    id: usize,
    index: usize,
    kind: BlockKind,
    branches: &[WorkPlan],
) -> Result<Option<Candidate>> {
    let mut frontiers = Vec::new();
    let mut continuing = Vec::with_capacity(branches.len());
    for branch in branches {
        let opened = frontiers.len();
        branch.collect_frontiers(&mut frontiers);
        continuing.push(frontiers.len() > opened);
    }
    if frontiers.len() < 2 {
        return Ok(None);
    }
    let ready = frontiers
        .iter()
        .map(|(_, state)| analysis.ready(state))
        .collect::<Result<Vec<_>>>()?;
    let Some(Some(shared)) = ready.first() else {
        return Ok(None);
    };
    if ready.iter().any(|candidate| candidate != &Some(*shared)) {
        return Ok(None);
    }
    if kind == BlockKind::Choice {
        choice::adjacent_branches(&continuing, &analysis.flow.blocks[index].outputs)?;
    }

    // The shared consumer's inputs come first and fix the yielded tuple
    // order; every other output joins them so alternative wires survive
    // for consumers further along the shared continuation.
    let inputs = analysis.flow.blocks[*shared]
        .inputs
        .iter()
        .map(|input| &input.ident);
    let outputs = analysis.flow.blocks.iter().flat_map(|block| &block.outputs);
    let mut wires = Vec::new();
    for ident in inputs.chain(outputs) {
        if wires.contains(ident) {
            continue;
        }
        let Some(first) = frontiers[0].1.available.get(ident) else {
            continue;
        };
        if frontiers[1..]
            .iter()
            .all(|(_, state)| state.available.contains_key(ident))
            && frontiers[1..]
                .iter()
                .any(|(_, state)| state.available[ident] != *first)
        {
            wires.push(ident.clone());
        }
    }
    Ok(Some(Candidate {
        branch: id,
        frontiers: frontiers.iter().map(|(id, _)| *id).collect(),
        wires,
    }))
}

fn public_branches(branches: Vec<WorkPlan>, joined: bool) -> Vec<Branch> {
    let yields = branches.iter().map(WorkPlan::yields).collect::<Vec<_>>();
    let continuing = joined || yields.contains(&true);
    branches
        .into_iter()
        .zip(yields)
        .map(|(plan, yields)| Branch {
            plan: Box::new(plan.into_plan()),
            early_return: continuing && !yields,
        })
        .collect()
}
