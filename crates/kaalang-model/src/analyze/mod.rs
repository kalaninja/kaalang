//! Walks every possible execution of a flow in source order, proving the
//! execution invariants of RFC 0001 and recording the executions, capture
//! dependencies, and convergence groups that the rest of the compiler relies on.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::ext::IdentExt;
use syn::{Error, Result};

use crate::model::{
    BlockKind, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup, Execution,
    ExecutionOutcome, Flow, ProducerId, WireMerge,
};

mod action;
mod break_block;
mod choice;
mod convergence;
mod end;
mod loop_block;
mod merge;
mod participation;
mod placement;
mod question;
mod return_block;

/// Walks the blocks in source order under every branch selection. Returns the
/// executions and convergence groups in canonical order, or the earliest
/// authored violation: a walk error first, then a block placed inside open
/// branches, then an execution that reaches the root boundary without `return`, then an unreachable block,
/// then a producer occurrence that no execution captures, then an invalid
/// branch-output continuation, then a block decided by independent questions
/// or choices, then an invalid wire merge, shared continuation, or loop route order.
pub(crate) fn flow(flow: &Flow) -> Result<(Vec<Execution>, Vec<ConvergenceGroup>, Vec<WireMerge>)> {
    let end = flow.blocks.len() - 1;
    debug_assert_eq!(flow.blocks[end].kind, BlockKind::End);
    let mut walk = Walk {
        flow,
        end,
        executions: BTreeSet::new(),
        error: None,
        incomplete: None,
    };
    walk.visit(
        0,
        State {
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
            repeats: BTreeSet::new(),
            loops: BTreeMap::new(),
        },
    );
    if let Some((_, error)) = walk.error {
        return Err(error);
    }

    let executions = walk.executions.into_iter().collect::<Vec<_>>();
    let mut merges = merge::collect(flow);
    let owners = merge::completion(flow, &executions, &mut merges);
    placement::flow(flow, &executions, &merges, &owners)?;
    if let Some(error) = walk.incomplete {
        return Err(error);
    }
    reachable(flow, &executions)?;
    captured(flow, &executions)?;
    branch_outputs(flow, &executions)?;
    let captures = executions
        .iter()
        .map(|execution| predecessors(flow, execution, &[]))
        .collect::<Vec<_>>();
    participation::flow(flow, &executions, &captures)?;
    let merges = merge::flow(flow, &executions, merges, owners)?;
    let precedence = executions
        .iter()
        .map(|execution| predecessors(flow, execution, &merges))
        .collect::<Vec<_>>();
    let convergence_groups = convergence::flow(flow, &executions, &precedence)?;
    loop_block::validate_routes(flow, &executions)?;
    Ok((executions, convergence_groups, merges))
}

/// Reports whether one producer occurrence provides its wire in this execution.
/// A branch output does so only when its own branch was selected.
fn produced(flow: &Flow, execution: &Execution, producer: ProducerId) -> bool {
    flow.produces(execution, producer)
}

/// The one question or choice that both executions run with different
/// outcomes, when every other one they both run agrees.
fn only_difference(left: &Execution, right: &Execution) -> Option<usize> {
    let mut differing = left.branches.iter().filter(|selection| {
        right
            .branches
            .iter()
            .any(|other| other.block == selection.block && other.branch != selection.branch)
    });
    let first = differing.next()?.block;
    differing.next().is_none().then_some(first)
}

/// Unfold only selections that change the observed outcome. Earlier converged
/// selections and later questions do not split its branch interval. Source
/// order puts deciding ancestors before descendants, so projected traces sort
/// in authored branch order.
pub(crate) fn branch_order<T: PartialEq>(executions: &[&Execution], outcomes: &[T]) -> Vec<usize> {
    let mut selectors = BTreeSet::new();
    for (first, execution) in executions.iter().enumerate() {
        for (second, other) in executions.iter().enumerate().skip(first + 1) {
            if outcomes[first] != outcomes[second]
                && let Some(selector) = only_difference(execution, other)
            {
                selectors.insert(selector);
            }
        }
    }
    let mut ordered = (0..executions.len()).collect::<Vec<_>>();
    ordered.sort_by_cached_key(|&index| {
        executions[index]
            .branches
            .iter()
            .filter(|selection| selectors.contains(&selection.block))
            .copied()
            .collect::<Vec<_>>()
    });
    ordered
}

/// Closes a relation held as one set per index: whatever `middle` relates to,
/// everything relating to `middle` relates to as well.
// ponytail: O(n³) in the number of indices; switch to a DAG walk if flows reach
// hundreds of blocks with many executions.
pub(crate) fn close(relation: &mut [BTreeSet<usize>]) {
    for middle in 0..relation.len() {
        let inherited = relation[middle].clone();
        for related in relation.iter_mut() {
            if related.contains(&middle) {
                related.extend(&inherited);
            }
        }
    }
}

/// The transitive predecessors of every block in one execution: its capture
/// dependencies together with the branch-local work and producers each wire
/// merge closes before its consumers.
fn predecessors(flow: &Flow, execution: &Execution, merges: &[WireMerge]) -> Vec<BTreeSet<usize>> {
    let blocks = flow.blocks.len();
    let mut preceding = vec![BTreeSet::new(); blocks];
    for &block in &execution.blocks {
        preceding[block].extend(flow.enclosing(block));
    }
    for dependency in &execution.dependencies {
        if let ProducerId::BlockOutput { block, .. } | ProducerId::CycleInput { block, .. } =
            dependency.producer
        {
            preceding[dependency.capture.block].insert(block);
        }
    }
    for merge in merges {
        let before = merge
            .producers
            .iter()
            .filter_map(|producer| match producer {
                ProducerId::BlockOutput { block, .. } => Some(*block),
                ProducerId::FlowInput(_) | ProducerId::CycleInput { .. } => None,
            })
            .chain(merge.before.iter().copied())
            .filter(|&block| execution.participates(block))
            .collect::<BTreeSet<_>>();
        for &block in &merge.after {
            if execution.participates(block) || block == blocks - 1 {
                preceding[block].extend(&before);
            }
        }
    }
    close(&mut preceding);
    preceding
}

/// One point of one execution, reached after the blocks above the current
/// source position have had their turn.
#[derive(Clone)]
struct State {
    /// Every wire this execution has provided, with the occurrence providing
    /// it. A bare capture does not remove it: Rust owns move checking.
    available: BTreeMap<Ident, ProducerId>,
    /// Every wire name this execution has produced, flow inputs included.
    produced: BTreeSet<Ident>,
    executed: BTreeSet<usize>,
    branches: BTreeSet<BranchSelection>,
    dependencies: BTreeSet<CaptureDependency>,
    repeats: BTreeSet<usize>,
    loops: BTreeMap<usize, LoopState>,
}

#[derive(Clone)]
struct LoopState {
    available: BTreeMap<Ident, ProducerId>,
    produced: BTreeSet<Ident>,
}

impl State {
    /// Records the capture dependencies of a participating block.
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
        }
        self.executed.insert(block);
    }
}

struct Walk<'a> {
    flow: &'a Flow,
    end: usize,
    executions: BTreeSet<Execution>,
    /// The earliest authored violation so far, keyed by block and occurrence.
    error: Option<((usize, usize), Error)>,
    /// An execution that reaches the root boundary without `return`. Reported
    /// after branch placement, which explains such an execution more directly.
    incomplete: Option<Error>,
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

    /// Gives each block from `index` on its turn, in source order. A block
    /// whose inputs this execution has all provided participates; the others
    /// belong to branches this execution did not select.
    fn visit(&mut self, index: usize, mut state: State) {
        if self.close_loops(index, &mut state) {
            return;
        }
        if index == self.end {
            self.finish(state);
            return;
        }
        let block = &self.flow.blocks[index];
        if !block
            .inputs
            .iter()
            .all(|input| state.available.contains_key(&input.ident))
        {
            self.visit(block.loop_end.unwrap_or(index + 1), state);
            return;
        }
        state.enter(self.flow, index);
        match block.kind {
            BlockKind::Action => action::visit(self, index, state),
            BlockKind::Question => question::visit(self, index, &state),
            BlockKind::Loop => loop_block::visit(self, index, state),
            BlockKind::Break => break_block::visit(self, index, state),
            BlockKind::Return => return_block::visit(self, index, state),
            BlockKind::Choice => choice::visit(self, index, &state),
            BlockKind::End => unreachable!("the end block closes the walk"),
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
                self.visit(block + 1, branch);
            }
        }
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

    /// Reaching the innermost body's boundary records a repeating summary.
    fn close_loops(&mut self, index: usize, state: &mut State) -> bool {
        let Some(header) = state
            .loops
            .keys()
            .rev()
            .copied()
            .find(|&header| self.flow.blocks[header].loop_end == Some(index))
        else {
            return false;
        };
        state.loops.remove(&header).expect("the iteration is open");
        let bindings = self.flow.blocks[header]
            .inputs
            .iter()
            .enumerate()
            .map(|(input, declaration)| {
                (
                    declaration
                        .binding
                        .clone()
                        .expect("a cycle capture declares a local binding"),
                    ProducerId::CycleInput {
                        block: header,
                        input,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        state.produced = bindings.keys().cloned().collect();
        state.available = bindings;
        state.repeats.insert(header);
        self.record(
            state.clone(),
            ExecutionOutcome::Repeat { loop_index: header },
        );
        true
    }

    /// Reaching the root boundary without an explicit return is invalid.
    fn finish(&mut self, _state: State) {
        self.incomplete
            .get_or_insert_with(|| end::missing_return(self.flow));
    }

    fn record(&mut self, state: State, outcome: ExecutionOutcome) {
        self.executions.insert(Execution {
            blocks: state.executed.into_iter().collect(),
            branches: state.branches.into_iter().collect(),
            dependencies: state.dependencies.into_iter().collect(),
            repeats: state.repeats.into_iter().collect(),
            outcome,
        });
    }
}

/// Every authored block participates in at least one execution.
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
/// execution unless its name begins with `_`. Only the action arm is reached in
/// practice: an uncaptured question or choice output leaves its execution
/// without a root return, and the walk reports that first.
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
            if ignored(&declaration.output_binding(output).ident)
                || captured(ProducerId::BlockOutput { block, output })
            {
                continue;
            }
            return Err(match declaration.kind {
                BlockKind::Action => action::uncaptured(name),
                BlockKind::Question => question::uncaptured(name),
                BlockKind::Choice => choice::uncaptured(name),
                BlockKind::Loop => loop_block::uncaptured(name),
                BlockKind::End | BlockKind::Break | BlockKind::Return => {
                    unreachable!("this kind declares no outputs")
                }
            });
        }
    }
    Ok(())
}

/// Every occurrence of a question or choice output is captured by at most one
/// block, which consumes it: a borrow or a second consumer would give the
/// selected branch a second continuation. The rule is occurrence-level, so a
/// repeated name is consumed by its implicit merge; its later captures use the
/// merged value. Otherwise, a branch output's consumer must capture it in
/// every execution selecting the output.
fn branch_outputs(flow: &Flow, executions: &[Execution]) -> Result<()> {
    // A repeated name is consumed by its implicit merge. Downstream captures
    // refer to the merged value, not to a raw question or choice output.
    let mut names = BTreeSet::new();
    let merged = flow
        .blocks
        .iter()
        .flat_map(|block| &block.outputs)
        .filter(|name| !names.insert(*name))
        .collect::<BTreeSet<_>>();
    let mut captures = BTreeMap::<ProducerId, BTreeSet<CaptureId>>::new();
    for dependency in executions
        .iter()
        .flat_map(|execution| &execution.dependencies)
    {
        if let ProducerId::BlockOutput { block, output } = dependency.producer
            && !merged.contains(&flow.blocks[block].outputs[output])
            && matches!(
                flow.blocks[block].kind,
                BlockKind::Question | BlockKind::Choice
            )
        {
            captures
                .entry(dependency.producer)
                .or_default()
                .insert(dependency.capture);
        }
    }
    let input = |capture: CaptureId| &flow.blocks[capture.block].inputs[capture.input];
    let borrowed = captures
        .values()
        .flatten()
        .filter(|&&capture| input(capture).borrowed)
        .map(|&capture| {
            (
                capture,
                "a kaalang branch output is consumed, never borrowed",
            )
        });
    let shared = captures
        .values()
        .filter_map(|captures| {
            let first = captures.first()?;
            captures.iter().find(|capture| capture.block != first.block)
        })
        .map(|&capture| {
            (
                capture,
                "a kaalang branch output is captured by at most one block",
            )
        });
    if let Some((capture, message)) = borrowed.chain(shared).min_by_key(|(capture, _)| *capture) {
        return Err(Error::new(input(capture).ident.span(), message));
    }
    // No fixture reaches the check below: a branch output left without its
    // consumer also leaves its execution without a root return, which the walk
    // reports first. Kept until that is proven rather than observed.
    let missing = captures.iter().filter_map(|(&producer, captures)| {
        let ProducerId::BlockOutput { block, output } = producer else {
            unreachable!("only branch outputs have been collected")
        };
        let &capture = captures.first()?;
        executions
            .iter()
            .any(|execution| {
                execution.branches.contains(&BranchSelection {
                    block,
                    branch: output,
                }) && !execution
                    .dependencies
                    .contains(&CaptureDependency { producer, capture })
            })
            .then_some(capture)
    });
    if let Some(capture) = missing.min() {
        return Err(Error::new(
            input(capture).ident.span(),
            "a kaalang branch output must reach its consumer whenever that output is selected",
        ));
    }
    Ok(())
}

/// A leading underscore permits a producer to have no consumer.
fn ignored(name: &Ident) -> bool {
    name.unraw().to_string().starts_with('_')
}
