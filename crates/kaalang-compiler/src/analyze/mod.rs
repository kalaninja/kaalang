//! Walks every possible execution of a flow in source order, proving the
//! execution invariants and recording the executions, capture
//! dependencies, and convergence groups that the rest of the compiler relies on.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use proc_macro2::Ident;
use syn::{Error, Result, ext::IdentExt};

use crate::model::{
    BlockKind, BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup, Execution,
    ExecutionOutcome, Flow, FlowKind, Passes, ProducerId, WireMerge,
};

mod action;
mod call;
mod choice;
mod comparison;
mod continue_block;
mod convergence;
pub(crate) mod cycle;
pub(crate) mod end;
mod export;
pub(crate) mod frame;
pub(crate) mod merge;
pub(crate) mod participation;
pub(crate) mod placement;
mod question;
mod return_block;

#[cfg(test)]
mod tests;

type FlowResult = (
    Vec<Execution>,
    Vec<ConvergenceGroup>,
    Vec<WireMerge>,
    Passes,
);

/// Enumerates executions in source order and derives canonical merges and groups.
/// Validation order below determines diagnostic priority.
pub(crate) fn flow(flow: &Flow, check_usage: bool) -> Result<FlowResult> {
    let walk = walk(flow);
    if let Some((_, error)) = walk.error {
        return Err(error);
    }

    let executions = walk.executions.into_iter().collect::<Vec<_>>();
    let frames = frame::Frames::of(flow, &executions);
    let mut merges = merge::collect(flow);
    let owners = merge::completion(flow, &frames, &mut merges);
    let ancestry = placement::ancestry(flow);
    placement::flow(flow, &executions, &merges, &owners, &ancestry, &frames)?;
    if let Some(error) = walk.incomplete {
        return Err(error);
    }
    reachable(flow, &executions)?;
    if check_usage {
        captured(flow, &executions)?;
    }
    branch_outputs(flow, &executions, &merges)?;
    let captures = executions
        .iter()
        .map(|execution| predecessors(flow, execution, &[]))
        .collect::<Vec<_>>();
    participation::flow(flow, &frames, &captures)?;
    let merges = merge::flow(flow, &frames, merges, owners, &ancestry)?;
    cycle::output_order(flow, &frames)?;
    let precedence = executions
        .iter()
        .map(|execution| predecessors(flow, execution, &merges))
        .collect::<Vec<_>>();
    let convergence_groups = convergence::flow(flow, &frames, &precedence, &ancestry)?;
    let passes = frames.into_passes();
    Ok((executions, convergence_groups, merges, passes))
}

/// Enumerates every finite execution summary of a flow in source order,
/// recording the first error the walk itself found.
fn walk(flow: &Flow) -> Walk<'_> {
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
            executed: BTreeSet::new(),
            branches: BTreeSet::new(),
            dependencies: BTreeSet::new(),
            cycles: BTreeMap::new(),
        },
    );
    walk
}

/// The parsed flow and its executions, before the passes that reject either.
/// Reference comparisons need the input those passes are given, including the
/// flows one of them goes on to refuse.
#[cfg(test)]
pub(crate) fn walked(function: &syn::ItemFn) -> Option<(Flow, Vec<Execution>)> {
    let mut flow = crate::parse::flow(function).ok()?;
    crate::scope::resolve(&mut flow).ok()?;
    crate::resolve::flow(&flow).ok()?;
    let executions = walk(&flow).executions.into_iter().collect();
    Some((flow, executions))
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
/// in authored branch order. A cycle seen as a black box sorts by its output
/// alone: the body route it carries tells executions apart but orders nothing.
pub(crate) fn branch_order<T: PartialEq>(executions: &[&Execution], outcomes: &[T]) -> Vec<usize> {
    let mut selectors = BTreeSet::new();
    let context = [outcomes.iter().map(Some).collect()];
    comparison::compare(executions, &context, |_, selector, first, second| {
        if outcomes[first] != outcomes[second] {
            selectors.insert(selector);
        }
    });
    let mut ordered = (0..executions.len()).collect::<Vec<_>>();
    ordered.sort_by_cached_key(|&index| {
        executions[index]
            .branches
            .iter()
            .filter(|selection| selectors.contains(&selection.block))
            .map(|selection| BranchSelection {
                block: selection.block,
                branch: frame::branch(selection.branch),
            })
            .collect::<Vec<_>>()
    });
    ordered
}

/// Closes a DAG relation, visiting related indices before their dependents.
/// Source order supplies this order for predecessors, its reverse for successors.
pub(crate) fn close(relation: &mut [BTreeSet<usize>], order: impl Iterator<Item = usize>) {
    let count = relation.len();
    let words = count.div_ceil(u64::BITS as usize);
    let mut reach: Vec<Option<Vec<u64>>> = vec![None; count];
    for index in order {
        let mut row = vec![0; words];
        for &related in &relation[index] {
            let inherited = reach[related]
                .as_ref()
                .expect("related indices must be closed first");
            row[related / u64::BITS as usize] |= 1 << (related % u64::BITS as usize);
            for (word, bits) in row.iter_mut().zip(inherited) {
                *word |= *bits;
            }
        }
        let mut related = Vec::new();
        for (word, mut bits) in row.iter().copied().enumerate() {
            while bits != 0 {
                related.push(word * u64::BITS as usize + bits.trailing_zeros() as usize);
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
        if let ProducerId::BlockOutput { block, .. } = dependency.producer {
            preceding[dependency.capture.block].insert(block);
        }
    }
    for merge in merges {
        let before = merge
            .producers
            .iter()
            .filter_map(|producer| match producer {
                ProducerId::BlockOutput { block, .. } => Some(*block),
                ProducerId::FlowInput(_) => None,
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
    executed: BTreeSet<usize>,
    branches: BTreeSet<BranchSelection>,
    dependencies: BTreeSet<CaptureDependency>,
    cycles: BTreeMap<usize, BTreeMap<Ident, ProducerId>>,
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
        if self.close_cycles(index, &state) {
            return;
        }
        if index == self.end {
            self.incomplete
                .get_or_insert_with(|| end::missing_return(self.flow));
            return;
        }
        let block = &self.flow.blocks[index];
        if block.transition_target.is_some() {
            let selected = self
                .flow
                .blocks
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    candidate.transition_target.is_some()
                        && state.available.contains_key(&candidate.inputs[0].ident)
                })
                .collect::<Vec<_>>();
            if selected.len() > 1 {
                self.report(
                    (index, 0),
                    Error::new(
                        self.flow.blocks[selected[1].0].span,
                        "a kaalang transition boundary selects more than one stage signal",
                    ),
                );
                return;
            }
            if let Some(input) = block
                .inputs
                .iter()
                .skip(1)
                .find(|input| !state.available.contains_key(&input.ident))
            {
                self.report(
                    (index, 0),
                    Error::new(input.alias.span(), "a common outer wire used by a kaalang stage must be available on every preparation route"),
                );
                return;
            }
        }
        if !block
            .inputs
            .iter()
            .filter(|input| !input.derived)
            .all(|input| state.available.contains_key(&input.ident))
        {
            self.visit(block.cycle_end.unwrap_or(index + 1), state);
            return;
        }
        // An inner capture cannot make entering the cycle conditional.
        if block.kind == BlockKind::Cycle
            && let Some((position, input)) = block
                .inputs
                .iter()
                .enumerate()
                .find(|(_, input)| input.derived && !state.available.contains_key(&input.ident))
        {
            self.report(
                (index, position),
                Error::new(
                    input.ident.span(),
                    "an outer wire captured inside a kaalang cycle must be available whenever the cycle is entered",
                ),
            );
            return;
        }
        state.enter(self.flow, index);
        match block.kind {
            BlockKind::Action => action::visit(self, index, state),
            BlockKind::Call => call::visit(self, index, state),
            BlockKind::Question => question::visit(self, index, &state),
            BlockKind::Cycle => cycle::visit(self, index, state),
            BlockKind::Export => export::visit(self, index, state),
            BlockKind::Continue => continue_block::visit(self, index, state),
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
        if state.available.contains_key(name) {
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

    /// An iteration still open at its body's boundary reached neither
    /// `continue` nor a declared output: repetition is authored, never implied.
    fn close_cycles(&mut self, index: usize, state: &State) -> bool {
        let Some(header) = state
            .cycles
            .keys()
            .rev()
            .copied()
            .find(|&header| self.flow.blocks[header].cycle_end == Some(index))
        else {
            return false;
        };
        // The route falls off at the body's end, after every block inside it.
        let message = if self.flow.blocks[header].outputs.is_empty() {
            "a route through this kaalang cycle reaches the end of its body; an outputless cycle repeats it with `continue`"
        } else {
            "a route through this kaalang cycle reaches the end of its body without a declared output; produce one of its outputs or repeat it with `continue`"
        };
        self.report(
            (index - 1, usize::MAX),
            Error::new(self.flow.blocks[header].span, message),
        );
        true
    }

    fn record(&mut self, state: State, outcome: ExecutionOutcome) {
        self.executions.insert(Execution {
            blocks: state.executed.into_iter().collect(),
            branches: state.branches.into_iter().collect(),
            dependencies: state.dependencies.into_iter().collect(),
            outcome,
        });
    }
}

/// The positions of the declared outputs of `header` an execution has already
/// produced inside its body.
fn produced_outputs<'s>(
    flow: &'s Flow,
    state: &'s State,
    header: usize,
) -> impl Iterator<Item = usize> + 's {
    flow.exports(header)
        .filter(move |&consumer| {
            state
                .available
                .contains_key(&flow.blocks[consumer].inputs[0].ident)
        })
        .map(move |consumer| flow.exported_output(consumer))
}

/// The authored name of the declared output a boundary consumer exports.
fn exported_name(flow: &Flow, consumer: usize) -> &Ident {
    let header = flow.blocks[consumer]
        .export_target
        .expect("a boundary consumer has a cycle");
    &flow.blocks[header]
        .output_binding(flow.exported_output(consumer))
        .ident
}

/// Every authored block participates in at least one execution.
fn reachable(flow: &Flow, executions: &[Execution]) -> Result<()> {
    let end = flow.blocks.len() - 1;
    match (0..end).find(|&block| {
        if matches!(flow.kind, FlowKind::Preparation)
            && flow.blocks[block].transition_target.is_some()
        {
            return false;
        }
        !executions
            .iter()
            .any(|execution| execution.participates(block))
    }) {
        Some(block) => Err(unreachable(flow, block)),
        None => Ok(()),
    }
}

pub(crate) fn unreachable(flow: &Flow, block: usize) -> Error {
    let message = if flow.blocks[block].kind == BlockKind::Export {
        format!(
            "no route through this kaalang cycle reaches the end of its body with its output `{}`",
            exported_name(flow, block)
        )
    } else {
        "this kaalang block is unreachable".to_owned()
    };
    Error::new(flow.blocks[block].span, message)
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
        if matches!(flow.kind, FlowKind::Stage { .. }) {
            continue;
        }
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
            && flow.blocks[block].branch_count() > 0
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
