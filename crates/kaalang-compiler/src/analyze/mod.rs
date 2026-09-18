//! Walks every possible execution of a flow in source order, proving the
//! execution invariants of RFC 0001 and recording the executions, capture
//! dependencies, and convergence groups that the rest of the compiler relies on.

use std::cmp::Ordering;
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
mod call;
mod choice;
mod convergence;
mod end;
mod loop_block;
mod merge;
mod participation;
mod placement;
mod question;
mod return_block;

#[cfg(test)]
mod tests;

/// Enumerates executions in source order and derives canonical merges and groups.
/// Validation order below determines diagnostic priority.
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
    branch_outputs(flow, &executions, &merges)?;
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
    Ok((executions, convergence_groups, merges))
}

/// The one question or choice that both executions run with different
/// outcomes, when every other one they both run agrees.
///
/// Walks the two selection lists together: each is sorted by block, and an
/// execution summary runs every block at most once, so a block appears in one
/// list at most once. A block only one of them selects belongs to a path the
/// other cut short and takes no part in the comparison.
fn only_difference(left: &Execution, right: &Execution) -> Option<usize> {
    let (mut left, mut right) = (left.branches.as_slice(), right.branches.as_slice());
    let mut difference = None;
    while let ([first, rest @ ..], [second, others @ ..]) = (left, right) {
        match first.block.cmp(&second.block) {
            Ordering::Less => left = rest,
            Ordering::Greater => right = others,
            Ordering::Equal => {
                if first.branch != second.branch {
                    if difference.is_some() {
                        return None;
                    }
                    difference = Some(first.block);
                }
                (left, right) = (rest, others);
            }
        }
    }
    difference
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

/// Closes a DAG relation, visiting related indices before their dependents.
/// Source order supplies this order for predecessors, its reverse for successors.
pub(crate) fn close(relation: &mut [BTreeSet<usize>], order: impl Iterator<Item = usize>) {
    let count = relation.len();
    let words = count.div_ceil(64);
    let mut reach: Vec<Option<Vec<u64>>> = vec![None; count];
    for index in order {
        let mut row = vec![0; words];
        for &related in &relation[index] {
            let inherited = reach[related]
                .as_ref()
                .expect("related indices must be closed first");
            row[related / 64] |= 1 << (related % 64);
            for (word, bits) in row.iter_mut().zip(inherited) {
                *word |= *bits;
            }
        }
        let mut related = Vec::new();
        for (word, mut bits) in row.iter().copied().enumerate() {
            while bits != 0 {
                related.push(word * 64 + bits.trailing_zeros() as usize);
                bits &= bits - 1;
            }
        }
        relation[index] = related.into_iter().collect();
        reach[index] = Some(row);
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
    close(&mut preceding, 0..blocks);
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
            self.incomplete
                .get_or_insert_with(|| end::missing_return(self.flow));
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
            BlockKind::Call => call::visit(self, index, state),
            BlockKind::Question => question::visit(self, index, &state),
            BlockKind::Loop => loop_block::visit(self, index, state),
            BlockKind::Break => break_block::visit(self, index, state),
            BlockKind::Return => return_block::visit(self, index, state),
            BlockKind::Choice => choice::visit(self, index, &state),
            BlockKind::End => unreachable!("the end block closes the walk"),
        }
    }

    /// A block that runs in place provides every output it declares, then the
    /// walk continues to the next block in source order.
    fn sequence(&mut self, block: usize, mut state: State) {
        let outputs = self.flow.blocks[block].outputs.len();
        if (0..outputs).all(|output| self.produce(&mut state, block, output)) {
            self.visit(block + 1, state);
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
        let bindings = self.flow.cycle_bindings(header);
        state.produced = bindings.keys().cloned().collect();
        state.available = bindings;
        state.repeats.insert(header);
        self.record(
            state.clone(),
            ExecutionOutcome::Repeat { loop_index: header },
        );
        true
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

/// Requires a capture in some execution for each producer not prefixed with `_`.
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
            // No fixture reaches the question or choice arm: an uncaptured
            // branch output also leaves its execution without a root return,
            // which the walk reports first. Kept because that is not proven.
            return Err(Error::new(
                name.span(),
                format!(
                    "every kaalang {} output must have a consumer",
                    crate::parse::noun(declaration.kind)
                ),
            ));
        }
    }
    Ok(())
}

/// Requires the first consumer whenever a branch output is selected. Later
/// consumers may be conditional. Repeated names are consumed by their merge.
fn branch_outputs(flow: &Flow, executions: &[Execution], merges: &[WireMerge]) -> Result<()> {
    // A repeated name is consumed by its implicit merge. Downstream captures
    // refer to the merged value, not to a raw question or choice output.
    let merged_wires = merges
        .iter()
        .map(|merge| &merge.wire)
        .collect::<BTreeSet<_>>();
    let mut captures = BTreeMap::<ProducerId, BTreeSet<CaptureId>>::new();
    for dependency in executions
        .iter()
        .flat_map(|execution| &execution.dependencies)
    {
        if let ProducerId::BlockOutput { block, output } = dependency.producer
            && !merged_wires.contains(&flow.blocks[block].outputs[output])
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
    // No fixture reaches the check below: a branch output left without its
    // first consumer also leaves its execution without a root return, which the
    // walk reports first. Kept until that is proven rather than observed.
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
            flow.blocks[capture.block].inputs[capture.input]
                .ident
                .span(),
            "a kaalang branch output must reach its first consumer whenever that output is selected",
        ));
    }
    Ok(())
}

/// A leading underscore permits a producer to have no consumer.
fn ignored(name: &Ident) -> bool {
    name.unraw().to_string().starts_with('_')
}
