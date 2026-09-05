//! Walks every possible execution of a flow, proving the execution invariants
//! of RFC 0001 and recording the executions and capture dependencies that the
//! rest of the compiler relies on.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{
    BlockKind, BranchSelection, CaptureDependency, CaptureId, Execution, Flow, ProducerId,
};

mod action;
mod choice;
mod convergence;
mod end;
mod question;

/// Explores every branch selection and every permitted order of ready
/// computational blocks. Returns the executions in canonical order, or the
/// earliest authored violation: a walk error first, then an unreachable block,
/// then a producer occurrence that no execution captures.
pub(crate) fn flow(flow: &Flow) -> Result<Vec<Execution>> {
    let end = flow.blocks.len() - 1;
    debug_assert_eq!(flow.blocks[end].kind, BlockKind::End);
    let mut walk = Walk {
        flow,
        end,
        visited: HashSet::new(),
        executions: BTreeSet::new(),
        error: None,
    };
    walk.visit(State {
        available: flow
            .flow_inputs
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), ProducerId::FlowInput(index)))
            .collect(),
        produced: flow.flow_inputs.iter().cloned().collect(),
        executed: BTreeSet::new(),
        branches: BTreeSet::new(),
        dependencies: BTreeSet::new(),
    });
    if let Some((_, error)) = walk.error {
        return Err(error);
    }

    let executions = walk.executions.into_iter().collect::<Vec<_>>();
    reachable(flow, &executions)?;
    captured(flow, &executions)?;
    convergence::flow(flow, &executions)?;
    Ok(executions)
}

/// One point of one execution. Every component is kept in stable order, so
/// two serial orders of independent blocks reach the same state and the walk
/// visits it once.
#[derive(Clone, PartialEq, Eq, Hash)]
struct State {
    /// Every wire a block may still capture, with the occurrence providing it.
    available: BTreeMap<Ident, ProducerId>,
    /// Every wire name this execution has produced, flow inputs included.
    produced: BTreeSet<Ident>,
    executed: BTreeSet<usize>,
    branches: BTreeSet<BranchSelection>,
    dependencies: BTreeSet<CaptureDependency>,
}

impl State {
    /// Records the capture dependencies of a block and consumes its bare inputs.
    fn enter(&mut self, flow: &Flow, block: usize) {
        for (index, input) in flow.blocks[block].inputs.iter().enumerate() {
            let producer = self.available[&input.ident];
            self.dependencies.insert(CaptureDependency {
                producer,
                capture: CaptureId {
                    block,
                    input: index,
                },
            });
            if !input.borrowed {
                self.available.remove(&input.ident);
            }
        }
        self.executed.insert(block);
    }
}

struct Walk<'a> {
    flow: &'a Flow,
    end: usize,
    visited: HashSet<State>,
    executions: BTreeSet<Execution>,
    /// The earliest authored violation so far, keyed by block and occurrence.
    error: Option<((usize, usize), Error)>,
}

impl Walk<'_> {
    fn report(&mut self, key: (usize, usize), error: Error) {
        if self
            .error
            .as_ref()
            .is_none_or(|(earliest, _)| key < *earliest)
        {
            self.error = Some((key, error));
        }
    }

    // ponytail: every subset of independent blocks is a distinct state, so the
    // walk is exponential in their number; flows are small enough that a
    // smarter partial-order reduction has not been worth it.
    fn visit(&mut self, state: State) {
        if !self.visited.insert(state.clone()) {
            return;
        }
        let ready = self.ready(&state);
        if let Some((block, wire)) = self.conflict(&ready) {
            self.report(
                (block, 0),
                Error::new(
                    self.flow.blocks[block].span,
                    format!(
                        "kaalang blocks that are ready for the same wire `{wire}` must both borrow it; add an explicit dependency"
                    ),
                ),
            );
            return;
        }
        if ready.is_empty() {
            self.finish(state);
            return;
        }

        for &block in &ready {
            let mut next = state.clone();
            next.enter(self.flow, block);
            match self.flow.blocks[block].kind {
                BlockKind::Action => action::visit(self, block, next),
                BlockKind::Question => question::visit(self, block, &next),
                BlockKind::Choice => choice::visit(self, block, &next),
                BlockKind::End => unreachable!("end is never in the ready set"),
            }
        }
    }

    /// Questions and choices each select exactly one output per successor.
    fn branch(&mut self, block: usize, state: &State) {
        for output in 0..self.flow.blocks[block].outputs.len() {
            let mut branch = state.clone();
            branch.branches.insert(BranchSelection {
                block,
                branch: output,
            });
            if self.produce(&mut branch, block, output) {
                self.visit(branch);
            }
        }
    }

    /// The unexecuted computational blocks whose inputs are all available.
    fn ready(&self, state: &State) -> Vec<usize> {
        (0..self.end)
            .filter(|block| {
                !state.executed.contains(block)
                    && self.flow.blocks[*block]
                        .inputs
                        .iter()
                        .all(|input| state.available.contains_key(&input.ident))
            })
            .collect()
    }

    /// Two ready blocks may share an input only when both borrow it. Reports
    /// the earliest authored block that conflicts with an earlier ready block.
    fn conflict(&self, ready: &[usize]) -> Option<(usize, Ident)> {
        for (position, &later) in ready.iter().enumerate() {
            for &earlier in &ready[..position] {
                for first in &self.flow.blocks[earlier].inputs {
                    for second in &self.flow.blocks[later].inputs {
                        if first.ident == second.ident && !(first.borrowed && second.borrowed) {
                            return Some((later, second.ident.clone()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Makes one output available unless this execution already produced its name.
    fn produce(&mut self, state: &mut State, block: usize, output: usize) -> bool {
        let name = &self.flow.blocks[block].outputs[output];
        if !state.produced.insert(name.clone()) {
            self.report(
                (block, output),
                Error::new(
                    name.span(),
                    "a kaalang wire must not be produced more than once in one execution",
                ),
            );
            return false;
        }
        state
            .available
            .insert(name.clone(), ProducerId::BlockOutput { block, output });
        true
    }

    /// Resolves the end block's inputs and records the completed execution.
    fn finish(&mut self, mut state: State) {
        if !end::arrive(self, &mut state) {
            return;
        }
        self.executions.insert(Execution {
            blocks: state.executed.into_iter().collect(),
            branches: state.branches.into_iter().collect(),
            dependencies: state.dependencies.into_iter().collect(),
        });
    }
}

/// Every computational block participates in at least one execution.
fn reachable(flow: &Flow, executions: &[Execution]) -> Result<()> {
    let end = flow.blocks.len() - 1;
    match (0..end).find(|&block| {
        !executions
            .iter()
            .any(|execution| execution.participates(block))
    }) {
        Some(block) => Err(Error::new(
            flow.blocks[block].span,
            "this kaalang block is unreachable",
        )),
        None => Ok(()),
    }
}

/// Every named producer occurrence has a capture dependency in at least one
/// execution unless its name begins with `_`.
fn captured(flow: &Flow, executions: &[Execution]) -> Result<()> {
    let captured = |producer: ProducerId| {
        executions.iter().any(|execution| {
            execution
                .dependencies
                .iter()
                .any(|dependency| dependency.producer == producer)
        })
    };

    for (index, input) in flow.flow_inputs.iter().enumerate() {
        if !ignored(input) && !captured(ProducerId::FlowInput(index)) {
            return Err(Error::new(
                input.span(),
                "every kaalang flow input must have a consumer",
            ));
        }
    }
    for (block, declaration) in flow.blocks.iter().enumerate() {
        for (output, name) in declaration.outputs.iter().enumerate() {
            if ignored(name) || captured(ProducerId::BlockOutput { block, output }) {
                continue;
            }
            return Err(match declaration.kind {
                BlockKind::Action => action::uncaptured(name),
                BlockKind::Question => question::uncaptured(name),
                BlockKind::Choice => choice::uncaptured(name),
                BlockKind::End => unreachable!("end declares no outputs"),
            });
        }
    }
    Ok(())
}

/// A leading underscore permits a producer to have no consumer.
fn ignored(name: &Ident) -> bool {
    name.to_string().starts_with('_')
}
