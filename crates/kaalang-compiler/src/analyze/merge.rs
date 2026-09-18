//! Records the implicit merge of every repeated output name and checks that
//! source order closes each merge's branch-local work before its consumers.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, Execution, ExecutionOutcome, Flow, ProducerId, WireMerge};

use super::only_difference;

pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    merges: Vec<WireMerge>,
    owners: Vec<Vec<(usize, Vec<usize>)>>,
) -> Result<Vec<WireMerge>> {
    let blocks = flow.blocks.len();
    let mut successors = vec![BTreeSet::new(); blocks + merges.len()];
    let mut groups = vec![BTreeMap::<Vec<usize>, BTreeSet<usize>>::new(); blocks];
    for dependency in executions
        .iter()
        .flat_map(|execution| &execution.dependencies)
    {
        if let ProducerId::BlockOutput { block, .. } = dependency.producer {
            successors[block].insert(dependency.capture.block);
        }
    }
    for ((index, merge), owners) in merges.iter().enumerate().zip(owners) {
        let node = blocks + index;
        for &producer in &merge.producers {
            let ProducerId::BlockOutput { block, .. } = producer else {
                unreachable!("a wire merge combines block outputs")
            };
            successors[block].insert(node);
        }
        successors[node].extend(&merge.after);
        for &block in &merge.before {
            successors[block].insert(node);
        }
        for (owner, branches) in owners {
            groups[owner].entry(branches).or_default().insert(node);
        }
    }

    for (block, groups) in groups.into_iter().enumerate() {
        if flow.blocks[block].kind == BlockKind::Choice {
            super::choice::validate_groups(
                &flow.blocks[block].outputs,
                &groups.into_iter().collect::<Vec<_>>(),
            )?;
        }
    }
    validate_order(flow, &merges, &successors)?;
    for merge in &merges {
        let producers = executions
            .iter()
            .map(|execution| {
                merge
                    .producers
                    .iter()
                    .copied()
                    .find(|&producer| flow.produces(execution, producer))
            })
            .collect::<Vec<_>>();
        validate_adjacency(flow, executions, merge, &producers)?;
    }
    Ok(merges)
}

/// Below this many executions every pair is compared once for the whole flow:
/// at most 2016 comparisons, less than indexing them would cost to set up.
const PAIRWISE_EXECUTIONS: usize = 64;

/// What the comparisons found for one merge: the selectors that choose between
/// its producers, and the blocks each selector decides.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Completion {
    owners: BTreeSet<usize>,
    decided: BTreeMap<usize, BTreeSet<usize>>,
}

impl Completion {
    /// One pair of executions that agree at every question and choice they both
    /// run but `selector`, and whether they reach different producers there.
    fn compare(
        &mut self,
        selector: usize,
        first: &Execution,
        second: &Execution,
        owning: bool,
        end: usize,
    ) {
        if owning {
            self.owners.insert(selector);
        }
        record_difference(
            self.decided.entry(selector).or_default(),
            first,
            second,
            end,
        );
    }

    /// The blocks that must finish before the merge: everything the selectors
    /// choosing between its producers decide.
    fn before(&self) -> Vec<usize> {
        self.owners
            .iter()
            .flat_map(|owner| &self.decided[owner])
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The branches of each owner taken anywhere in the merge's context, which
    /// is wider than the pairs that named the owner.
    fn groups(
        &self,
        executions: &[&Execution],
        context: &[Option<ProducerId>],
    ) -> Vec<(usize, Vec<usize>)> {
        self.owners
            .iter()
            .map(|&owner| {
                let branches = executions
                    .iter()
                    .zip(context)
                    .filter(|(_, producer)| producer.is_some())
                    .flat_map(|(execution, _)| &execution.branches)
                    .filter(|selection| selection.block == owner)
                    .map(|selection| selection.branch)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                (owner, branches)
            })
            .collect()
    }
}

/// Records branch completion before placement checks common work. Validation
/// of the resulting merge order follows the execution and capture diagnostics.
///
/// Every merge reads the same execution pairs, so the comparisons run once for
/// the whole flow and each one is handed to the merges both its executions
/// produce.
pub(super) fn completion(
    flow: &Flow,
    executions: &[Execution],
    merges: &mut [WireMerge],
) -> Vec<Vec<(usize, Vec<usize>)>> {
    if merges.is_empty() {
        return Vec::new();
    }
    let end = flow.blocks.len() - 1;
    let (producing, context) = context(flow, executions, merges);
    let mut found = vec![Completion::default(); merges.len()];
    if producing.len() <= PAIRWISE_EXECUTIONS {
        compare_every_pair(&producing, &context, &mut found, end);
    } else {
        compare_compatible(&producing, &context, &mut found, end);
    }
    merges
        .iter_mut()
        .zip(&found)
        .zip(&context)
        .map(|((merge, found), context)| {
            merge.before = found.before();
            found.groups(&producing, context)
        })
        .collect()
}

/// The executions that produce at least one merge, and the producer each of
/// them reaches for each merge. `None` says that execution produces none of
/// that one's alternatives.
///
/// An execution outside every merge's context settles no merge's order, so
/// dropping it here keeps it out of the comparisons and out of the count the
/// strategy is chosen by. Both come back aligned, which is what lets the
/// comparisons index a context by an execution's position.
fn context<'a>(
    flow: &Flow,
    executions: &'a [Execution],
    merges: &[WireMerge],
) -> (Vec<&'a Execution>, Vec<Vec<Option<ProducerId>>>) {
    let mut producing = Vec::new();
    let mut context = vec![Vec::new(); merges.len()];
    let mut reached = Vec::with_capacity(merges.len());
    for execution in executions {
        reached.clear();
        reached.extend(merges.iter().map(|merge| {
            merge
                .producers
                .iter()
                .copied()
                .find(|&producer| flow.produces(execution, producer))
        }));
        if reached.iter().all(Option::is_none) {
            continue;
        }
        producing.push(execution);
        for (column, producer) in context.iter_mut().zip(&reached) {
            column.push(*producer);
        }
    }
    (producing, context)
}

/// Compares every pair of executions. Bounded by [`PAIRWISE_EXECUTIONS`], this
/// stays under 2016 comparisons and needs nothing built up front.
fn compare_every_pair(
    executions: &[&Execution],
    context: &[Vec<Option<ProducerId>>],
    found: &mut [Completion],
    end: usize,
) {
    for (position, first) in executions.iter().enumerate() {
        for (offset, second) in executions[position + 1..].iter().enumerate() {
            let Some(selector) = only_difference(first, second) else {
                continue;
            };
            let other = position + 1 + offset;
            for (found, context) in found.iter_mut().zip(context) {
                let (Some(left), Some(right)) = (context[position], context[other]) else {
                    continue;
                };
                found.compare(selector, first, second, left != right, end);
            }
        }
    }
}

/// Finds the same differences without looking at every pair.
///
/// Two executions differ at one selector alone only when both run it and agree
/// at every other selector they both run. Grouping executions by the selectors
/// they run and sorting each group pair by the shared selectors but one leaves
/// exactly those candidates adjacent: one compatible group per run of equal
/// projections, holding every pair that differs at that one selector.
///
/// A compatible group is still answered without pairing all of it. Comparisons
/// with up to two representatives per side keep each group's differing-branch
/// pairs connected, and connection is all either answer needs. Equal producers
/// carry along a connected component, so a component that hides a producer
/// difference cannot have all of its representative comparisons agree. Block
/// participation composes the same way: the blocks two executions disagree
/// about are a subset of what each disagrees about with anything between them,
/// so the comparisons spanning a component already union to what pairing all
/// of it would give.
///
/// Each group pair picks between the index and its own pairs by what they
/// cost, so the index is built only where it wins. Taking a group pair's own
/// pairs is never worse than scanning every pair of the flow, because the
/// group pairs partition exactly those, minus the ones that share no selector.
///
/// The remaining bound is the group pairs themselves: a flow whose executions
/// run many different selector sets still combines them pairwise.
fn compare_compatible(
    executions: &[&Execution],
    context: &[Vec<Option<ProducerId>>],
    found: &mut [Completion],
    end: usize,
) {
    let shapes = shapes(executions);
    let mut common = Vec::new();
    let mut members = Vec::new();
    let mut projection = Vec::new();
    let mut order = Vec::new();
    let mut sides = Sides::default();
    for (position, (selectors, group)) in shapes.iter().enumerate() {
        for (offset, (others, other)) in shapes[position..].iter().enumerate() {
            let crossing = offset > 0;
            let (left, right) = if crossing {
                (group.as_slice(), other.as_slice())
            } else {
                (group.as_slice(), &group[..0])
            };
            let pairs = if crossing {
                left.len() * right.len()
            } else {
                left.len() * (left.len() - 1) / 2
            };
            if pairs == 0 {
                continue;
            }
            shared(selectors, others, &mut common);
            if common.is_empty() {
                continue;
            }
            // Sorting a run once per shared selector, on a comparison that
            // walks the others, only pays off when a group pair holds many
            // more members than the selectors they share. Below that its own
            // pairs cost less, and taking them is never worse than scanning
            // every pair of the flow: the group pairs partition those.
            let width = common.len();
            let reach = left.len() + right.len();
            if width * reach * reach.ilog2().max(1) as usize >= pairs {
                for (index, &first) in left.iter().enumerate() {
                    let rest = if crossing { right } else { &left[index + 1..] };
                    for &second in rest {
                        let Some(selector) = only_difference(executions[first], executions[second])
                        else {
                            continue;
                        };
                        for (found, context) in found.iter_mut().zip(context) {
                            let (Some(reached), Some(alternative)) =
                                (context[first], context[second])
                            else {
                                continue;
                            };
                            found.compare(
                                selector,
                                executions[first],
                                executions[second],
                                reached != alternative,
                                end,
                            );
                        }
                    }
                }
                continue;
            }
            members.clear();
            members.extend(group);
            if crossing {
                members.extend(other);
            }
            projection.clear();
            for &member in &members {
                project(executions[member], &common, &mut projection);
            }
            for (column, &selector) in common.iter().enumerate() {
                let compatible = Compatible {
                    executions,
                    members: &members,
                    projection: &projection,
                    width,
                    column,
                    split: group.len(),
                    selector,
                    crossing,
                };
                order.clear();
                order.extend(0..members.len());
                order.sort_unstable_by(|&first, &second| compatible.compare(first, second));
                let mut start = 0;
                while start < order.len() {
                    let mut finish = start + 1;
                    while finish < order.len()
                        && compatible.compare(order[start], order[finish]).is_eq()
                    {
                        finish += 1;
                    }
                    for (found, context) in found.iter_mut().zip(context) {
                        compatible.record(&order[start..finish], context, found, end, &mut sides);
                    }
                    start = finish;
                }
            }
        }
    }
}

/// Executions grouped by the questions and choices they run, in block order.
/// Two executions can differ at one selector only when both run it, so every
/// pair worth comparing lies inside one pair of these groups.
fn shapes(executions: &[&Execution]) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut shapes = BTreeMap::<Vec<usize>, Vec<usize>>::new();
    for (index, execution) in executions.iter().enumerate() {
        shapes
            .entry(
                execution
                    .branches
                    .iter()
                    .map(|selection| selection.block)
                    .collect(),
            )
            .or_default()
            .push(index);
    }
    shapes.into_iter().collect()
}

/// The selectors both groups run. Both lists are sorted, so one walk finds them.
/// The buffer is reused across group pairs, of which a flow has many.
fn shared(first: &[usize], second: &[usize], shared: &mut Vec<usize>) {
    shared.clear();
    let (mut first, mut second) = (first, second);
    while let ([left, rest @ ..], [right, others @ ..]) = (first, second) {
        match left.cmp(right) {
            Ordering::Less => first = rest,
            Ordering::Greater => second = others,
            Ordering::Equal => {
                shared.push(*left);
                (first, second) = (rest, others);
            }
        }
    }
}

/// Appends the branches one execution takes at `shared`, in that order. Both
/// lists are sorted by block and `shared` only names selectors this execution
/// runs, so one walk finds every branch.
fn project(execution: &Execution, shared: &[usize], projection: &mut Vec<usize>) {
    let mut branches = execution.branches.iter();
    for &selector in shared {
        let selection = branches
            .find(|selection| selection.block == selector)
            .expect("a shared selector runs in both groups");
        projection.push(selection.branch);
    }
}

/// Scratch for one compatible group: its members on each side of the group
/// pair, kept between groups so the walk allocates nothing per group.
#[derive(Default)]
struct Sides {
    left: Vec<usize>,
    right: Vec<usize>,
}

/// One pair of selector-set groups, compared at one selector they share.
/// Members index `members`, which holds the left group and, when the pair
/// crosses two groups, the right one after `split`.
struct Compatible<'a> {
    executions: &'a [&'a Execution],
    members: &'a [usize],
    projection: &'a [usize],
    width: usize,
    column: usize,
    split: usize,
    selector: usize,
    crossing: bool,
}

impl Compatible<'_> {
    /// Orders members by every shared selector but the one under test. Equal
    /// members are the ones that may differ at it and nowhere else.
    fn compare(&self, first: usize, second: usize) -> Ordering {
        let (first, second) = (self.row(first), self.row(second));
        first[..self.column]
            .cmp(&second[..self.column])
            .then_with(|| first[self.column + 1..].cmp(&second[self.column + 1..]))
    }

    fn row(&self, member: usize) -> &[usize] {
        &self.projection[member * self.width..(member + 1) * self.width]
    }

    fn branch(&self, member: usize) -> usize {
        self.projection[member * self.width + self.column]
    }

    fn execution(&self, member: usize) -> &Execution {
        self.executions[self.members[member]]
    }

    /// Hands one merge the comparisons of one compatible group. Members of the
    /// group already agree everywhere else, so a differing branch at the
    /// selector is the whole test.
    fn record(
        &self,
        group: &[usize],
        context: &[Option<ProducerId>],
        found: &mut Completion,
        end: usize,
        sides: &mut Sides,
    ) {
        sides.left.clear();
        sides.right.clear();
        for &member in group {
            if context[self.members[member]].is_none() {
                continue;
            }
            if member < self.split {
                sides.left.push(member);
            } else {
                sides.right.push(member);
            }
        }
        if self.crossing {
            for (members, others) in [(&sides.left, &sides.right), (&sides.right, &sides.left)] {
                let representatives = self.representatives(others);
                for &member in members {
                    for other in representatives.into_iter().flatten() {
                        if self.branch(member) != self.branch(other) {
                            self.pair(member, other, context, found, end);
                        }
                    }
                }
            }
        } else {
            let [Some(first), Some(second)] = self.representatives(&sides.left) else {
                return;
            };
            for &member in &sides.left {
                let partner = if self.branch(member) == self.branch(first) {
                    second
                } else {
                    first
                };
                self.pair(member, partner, context, found, end);
            }
        }
    }

    /// Up to two members taking different branches at the selector. Any third
    /// branch differs from one of them, which is what keeps every member of a
    /// compatible group connected to the rest through valid comparisons.
    fn representatives(&self, members: &[usize]) -> [Option<usize>; 2] {
        let mut representatives = [None, None];
        for &member in members {
            match representatives {
                [None, _] => representatives[0] = Some(member),
                [Some(first), None] if self.branch(first) != self.branch(member) => {
                    representatives[1] = Some(member);
                }
                _ => {}
            }
        }
        representatives
    }

    fn pair(
        &self,
        first: usize,
        second: usize,
        context: &[Option<ProducerId>],
        found: &mut Completion,
        end: usize,
    ) {
        found.compare(
            self.selector,
            self.execution(first),
            self.execution(second),
            context[self.members[first]] != context[self.members[second]],
            end,
        );
    }
}

/// Producer routes occupy one interval in authored branch order.
fn validate_adjacency(
    flow: &Flow,
    executions: &[Execution],
    merge: &WireMerge,
    producers: &[Option<ProducerId>],
) -> Result<()> {
    // A repeating summary stops at its loop tail. It cannot separate routes
    // of a merge outside that loop because it never reaches the merge.
    let has_consumer = !merge.after.is_empty();
    let points = if has_consumer {
        merge.after.clone()
    } else {
        merge
            .producers
            .iter()
            .filter_map(|producer| match producer {
                ProducerId::BlockOutput { block, .. } => Some(*block),
                ProducerId::FlowInput(_) | ProducerId::CycleInput { .. } => None,
            })
            .collect::<Vec<_>>()
    };
    let (executions, producers): (Vec<_>, Vec<_>) = executions
        .iter()
        .zip(producers)
        .filter(|(execution, _)| match execution.outcome {
            ExecutionOutcome::Return { .. } => true,
            ExecutionOutcome::Repeat { loop_index } => points.iter().any(|&block| {
                (has_consumer && block == loop_index)
                    || flow.enclosing(block).any(|parent| parent == loop_index)
            }),
        })
        .map(|(execution, producer)| (execution, *producer))
        .unzip();
    let ordered = super::branch_order(&executions, &producers);
    let first = ordered.iter().position(|&index| producers[index].is_some());
    let last = ordered
        .iter()
        .rposition(|&index| producers[index].is_some());
    if let (Some(first), Some(last)) = (first, last)
        && ordered[first..=last]
            .iter()
            .any(|&index| producers[index].is_none())
    {
        let wire = flow.wire_name(&merge.wire);
        return Err(Error::new(
            merge.wire.span(),
            format!(
                "branches reaching the `{wire}` wire merge must be adjacent, including nested branches"
            ),
        ));
    }
    Ok(())
}

/// Checks the combined capture and merge order before any consumer can use it
/// for lowering or drawing, and that source order already closes each merge's
/// branch-local work above the blocks capturing the merged wire.
fn validate_order(flow: &Flow, merges: &[WireMerge], successors: &[BTreeSet<usize>]) -> Result<()> {
    // A block that both waits for a merge and must finish before it names the
    // merged wire for a value that never left its own branch. Such a block
    // always closes a cycle too, so this pass runs first: it names the mistake
    // even when an unrelated merge closes an earlier cycle.
    if let Some(input) = merges.iter().find_map(|merge| local_capture(flow, merge)) {
        return Err(Error::new(
            input.span(),
            "a kaalang wire with alternative producers merges before every capture; a branch-local value needs its own name",
        ));
    }
    for (index, merge) in merges.iter().enumerate() {
        let merge_node = flow.blocks.len() + index;
        if let Some(block) = reachable(successors, merge_node)
            .into_iter()
            .find(|&node| successors[node].contains(&merge_node))
        {
            let wire = flow.wire_name(&merge.wire);
            return Err(Error::new(
                flow.blocks[block].span,
                format!(
                    "this kaalang block must finish before the `{wire}` wire merge, but it waits for a value from after that merge"
                ),
            ));
        }
    }
    let late = merges
        .iter()
        .filter_map(|merge| {
            let consumer = *merge.after.first()?;
            let block = *merge.before.iter().find(|&&block| block > consumer)?;
            Some((block, &merge.wire))
        })
        .min_by_key(|&(block, _)| block);
    if let Some((block, wire)) = late {
        let wire = flow.wire_name(wire);
        return Err(Error::new(
            flow.blocks[block].span,
            format!(
                "this kaalang block must finish before the `{wire}` wire merge; declare it above the blocks that capture the merged wire"
            ),
        ));
    }
    Ok(())
}

/// The input of the earliest branch-local block that captures the merged wire
/// itself. Such a block would have to run both before and after the merge.
fn local_capture<'a>(flow: &'a Flow, merge: &WireMerge) -> Option<&'a Ident> {
    merge.before.iter().find_map(|&block| {
        flow.blocks[block]
            .inputs
            .iter()
            .find(|input| input.ident == merge.wire)
            .map(|input| &input.ident)
    })
}

/// Records the blocks below `end` that exactly one of two executions runs.
/// Both block lists are sorted, so one walk over the pair replaces a search
/// per block of the flow.
fn record_difference(
    decided: &mut BTreeSet<usize>,
    first: &Execution,
    second: &Execution,
    end: usize,
) {
    let mut decide = |block: usize| {
        if block < end {
            decided.insert(block);
        }
    };
    let (mut first, mut second) = (first.blocks.as_slice(), second.blocks.as_slice());
    while let ([left, rest @ ..], [right, others @ ..]) = (first, second) {
        match left.cmp(right) {
            Ordering::Less => {
                decide(*left);
                first = rest;
            }
            Ordering::Greater => {
                decide(*right);
                second = others;
            }
            Ordering::Equal => (first, second) = (rest, others),
        }
    }
    first.iter().chain(second).copied().for_each(decide);
}

/// Every repeated output name defines one merge, in first-producer order.
pub(super) fn collect(flow: &Flow) -> Vec<WireMerge> {
    let mut producers = BTreeMap::<_, Vec<_>>::new();
    for (block, declaration) in flow.blocks.iter().enumerate() {
        for (output, wire) in declaration.outputs.iter().enumerate() {
            producers
                .entry(wire.clone())
                .or_default()
                .push(ProducerId::BlockOutput { block, output });
        }
    }
    let mut merges = producers
        .into_iter()
        .filter(|(_, producers)| producers.len() > 1)
        .map(|(wire, producers)| WireMerge {
            after: flow
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(block, declaration)| {
                    declaration
                        .inputs
                        .iter()
                        .any(|input| input.ident == wire)
                        .then_some(block)
                })
                .collect(),
            wire,
            producers,
            before: Vec::new(),
        })
        .collect::<Vec<_>>();
    merges.sort_by_key(|merge| merge.producers[0]);
    merges
}

fn reachable(successors: &[BTreeSet<usize>], start: usize) -> BTreeSet<usize> {
    let mut reached = BTreeSet::new();
    let mut pending = successors[start].iter().copied().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        if reached.insert(node) {
            pending.extend(&successors[node]);
        }
    }
    reached
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use proc_macro2::Ident;
    use syn::{ItemFn, parse_quote};

    use super::{
        Completion, collect, compare_compatible, compare_every_pair, completion, context,
        only_difference,
    };
    use crate::tests::message as error;
    use crate::{Execution, Flow, ProducerId, WireMerge, build};

    fn merge(function: &ItemFn, wire: &str) -> WireMerge {
        let model = build(function).expect("the flow is valid");
        model
            .analysis
            .merges
            .into_iter()
            .find(|merge| merge.wire == wire)
            .unwrap_or_else(|| panic!("the `{wire}` wire merges"))
    }

    fn output(block: usize, output: usize) -> ProducerId {
        ProducerId::BlockOutput { block, output }
    }

    #[test]
    fn an_unrelated_earlier_selection_does_not_split_a_later_partial_merge() {
        let function: ItemFn = parse_quote! {
            fn valid(flag: bool, mode: u8) -> u8 {
                #[question("Choose the seed.")]
                let (yes, no) = |flag| { flag };
                #[action("First seed.")] let seed = |yes| { 1 };
                #[action("Second seed.")] let seed = |no| { 2 };
                #[choice("Choose the work.")]
                #[case("First.")]
                #[case("Second.")]
                #[case("Finish.")]
                let (first, second, finish) = |mode| {
                    match mode { 0 => (), 1 => (), _ => () }
                };
                #[action("First value.")] let shared = |first| { 10 };
                #[action("Second value.")] let shared = |second| { 20 };
                #[action("Finish early.")] let end = |finish, seed| { seed };
                #[action("Use the value.")] let end = |shared, seed| { shared + seed };
                |end| return end;
            }
        };
        assert_eq!(
            merge(&function, "shared").producers,
            [output(4, 0), output(5, 0)]
        );
    }

    #[test]
    fn an_unused_merge_cannot_skip_a_nested_branch() {
        let function: ItemFn = parse_quote! {
            fn invalid(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested branch?")]
                let (nested, direct) = |outer| { outer };
                #[question("Add the marker?")]
                let (mark, skip) = |nested, inner| { inner };
                #[action("Mark the first branch.")]
                let (_marker, end) = |mark| { ((), 1) };
                #[action("Leave the marker absent.")]
                let end = |skip| { 2 };
                #[action("Mark the last branch.")]
                let (_marker, end) = |direct| { ((), 3) };
                |end| return end;
            }
        };
        assert_eq!(
            error(&function),
            "branches reaching the `_marker` wire merge must be adjacent, including nested branches"
        );
    }

    #[test]
    fn an_inner_merge_may_precede_the_enclosing_merge() {
        let function: ItemFn = parse_quote! {
            fn valid(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested branch?")]
                let (nested, direct) = |outer| { outer };
                #[question("Choose the nested value.")]
                let (yes, no) = |nested, inner| { inner };
                #[action("Build the yes value.")]
                let refined = |yes| { 1 };
                #[action("Build the no value.")]
                let refined = |no| { 2 };
                #[action("Finish the nested branch.")]
                let shared = |refined| { refined + 10 };
                #[action("Build the direct value.")]
                let shared = |direct| { 3 };
                |shared| return shared;
            }
        };
        assert_eq!(merge(&function, "refined").after, [4]);
        assert_eq!(merge(&function, "shared").after, [6]);
    }

    #[test]
    fn a_partial_continuation_may_feed_a_wider_merge() {
        let function: ItemFn = parse_quote! {
            fn valid(a: bool, b: bool, c: bool) -> bool {
                #[question("a")]
                let (check_b, false_result) = |a| { a };
                #[question("b")]
                let (check_c, false_result) = |check_b, b| { b };
                #[question("c")]
                let (true_result, false_result) = |check_c, c| { c };
                #[action("Build true.")]
                let combined = |true_result| { true };
                #[action("Build false.")]
                let combined = |false_result| { false };
                #[action("Use the wider merge.")]
                let result = |combined| { combined };
                |result| return result;
            }
        };

        let model = build(&function).expect("a partial continuation may feed a wider merge");
        let false_result = model
            .analysis
            .merges
            .iter()
            .find(|merge| merge.wire == "false_result")
            .expect("the false routes merge");
        let combined = model
            .analysis
            .merges
            .iter()
            .find(|merge| merge.wire == "combined")
            .expect("the partial and direct routes merge");
        assert_eq!(false_result.after, [4]);
        assert_eq!(combined.after, [5]);
    }

    #[test]
    fn a_repeat_route_does_not_split_a_merge_after_its_loop() {
        let source = r#"
            fn valid(run: bool, done: bool, value: u8) -> u8 {
                #[question("Run the loop?")]
                let (enter, fallback) = |run| { run };
                #[cycle("Wait until done.")]
                let result = |enter, done, value| {
                    #[question("Done?")]
                    let (leave, _again) = |done| { done };
                    |leave, value| break value;
                };
                #[action("Use the fallback.")]
                let result = |fallback| { 2 };
                |result| return result;
            }
        "#;
        for source in [
            source.to_owned(),
            source.replace("(leave, _again)", "(_again, leave)"),
        ] {
            let function = syn::parse_str::<ItemFn>(&source).expect("the flow parses");
            build(&function).expect("a repeat does not reach the merge after its loop");
        }
    }

    #[test]
    fn a_branch_output_and_an_action_output_merge_before_every_capture() {
        let function: ItemFn = parse_quote! {
            fn valid(condition: bool) -> u8 {
                #[question("Which value?")]
                let (shared, no) = |condition| { condition };
                #[action("Produce the other value.")]
                let shared = |no| { () };
                #[action("Use the merged value.")]
                let end = |shared| { 0 };
                |end| return end;
            }
        };
        let merge = merge(&function, "shared");
        assert_eq!(merge.producers, [output(0, 0), output(1, 0)]);
        assert_eq!(merge.before, [1]);
    }

    #[test]
    fn a_merged_branch_output_can_be_borrowed_then_consumed() {
        let function: ItemFn = parse_quote! {
            fn valid(condition: bool) -> u8 {
                #[question("Which value?")]
                let (shared, no) = |condition| { condition };
                #[action("Produce the other value.")]
                let shared = |no| { () };
                #[action("Borrow the merged value.")]
                let ready = |&shared| { () };
                #[action("Consume it after the borrow.")]
                let end = |shared, ready| { 0 };
                |end| return end;
            }
        };
        assert_eq!(merge(&function, "shared").before, [1]);
    }

    #[test]
    fn unused_alternative_outputs_still_record_a_merge() {
        let function: ItemFn = parse_quote! {
            fn valid(condition: bool) -> u8 {
                #[question("Which marker?")]
                let (yes, no) = |condition| { condition };
                #[action("First marker.")]
                let (_marker, end) = |yes| { ((), 1) };
                #[action("Second marker.")]
                let (_marker, end) = |no| { ((), 2) };
                |end| return end;
            }
        };
        let merge = merge(&function, "_marker");
        assert_eq!(merge.wire, Ident::new("_marker", merge.wire.span()));
        assert_eq!(merge.producers, [output(1, 0), output(2, 0)]);
        assert_eq!(merge.before, [1, 2]);
    }

    #[test]
    fn unused_outputs_cannot_merge_nonadjacent_cases() {
        let function: ItemFn = parse_quote! {
            fn invalid(value: u8) -> u8 {
                #[choice("Which branch?")]
                #[case("First")]
                #[case("Between")]
                #[case("Last")]
                let (a, b, c) = |value| {
                    match value { 0 => (), 1 => (), _ => () }
                };
                #[action("First marker.")] let (_marker, end) = |a| { ((), 1) };
                #[action("Middle result.")] let end = |b| { 2 };
                #[action("Last marker.")] let (_marker, end) = |c| { ((), 3) };
                |end| return end;
            }
        };
        assert_eq!(
            error(&function),
            "branches in a kaalang choice convergence group must be adjacent"
        );
    }

    #[test]
    fn unused_outputs_cannot_form_crossing_merge_groups() {
        let function: ItemFn = parse_quote! {
            fn invalid(value: u8) -> u8 {
                #[choice("Which branch?")]
                #[case("Left")]
                #[case("Both")]
                #[case("Right")]
                let (a, b, c) = |value| {
                    match value { 0 => (), 1 => (), _ => () }
                };
                #[action("Left marker.")] let (_left, end) = |a| { ((), 1) };
                #[action("Both markers.")] let (_left, _right, end) = |b| { ((), (), 2) };
                #[action("Right marker.")] let (_right, end) = |c| { ((), 3) };
                |end| return end;
            }
        };
        assert_eq!(
            error(&function),
            "kaalang choice convergence groups must be disjoint or nested"
        );
    }

    /// The pairwise algorithm the index replaced, kept as the comparison
    /// baseline. Written the way it was, not through `record_difference`, so a
    /// mistake in either helper shows up as a disagreement.
    fn reference(
        flow: &Flow,
        executions: &[Execution],
        merge: &WireMerge,
    ) -> (Vec<usize>, Vec<(usize, Vec<usize>)>) {
        let end = flow.blocks.len() - 1;
        let context = executions
            .iter()
            .filter_map(|execution| {
                merge.producers.iter().find_map(|&producer| {
                    flow.produces(execution, producer)
                        .then_some((execution, producer))
                })
            })
            .collect::<Vec<_>>();
        let mut owners = BTreeSet::new();
        let mut decided = BTreeMap::<_, BTreeSet<_>>::new();
        for (position, (first, first_producer)) in context.iter().enumerate() {
            for (second, second_producer) in &context[position + 1..] {
                let Some(question) = only_difference(first, second) else {
                    continue;
                };
                if first_producer != second_producer {
                    owners.insert(question);
                }
                decided.entry(question).or_default().extend(
                    (0..end)
                        .filter(|&block| first.participates(block) != second.participates(block)),
                );
            }
        }
        let before = owners
            .iter()
            .flat_map(|owner| &decided[owner])
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let groups = owners
            .into_iter()
            .map(|owner| {
                let branches = context
                    .iter()
                    .flat_map(|(execution, _)| &execution.branches)
                    .filter(|selection| selection.block == owner)
                    .map(|selection| selection.branch)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                (owner, branches)
            })
            .collect();
        (before, groups)
    }

    /// Checks one flow three ways: what `completion` decides against the
    /// reference, the index against the pairwise scan on the same executions
    /// whatever their number, and that no merge waits for the implicit end.
    fn agrees(name: &str, function: &ItemFn) {
        let Some((flow, executions)) = crate::analyze::walked(function) else {
            panic!("{name}: the flow parses and resolves")
        };
        let end = flow.blocks.len() - 1;
        let mut merges = collect(&flow);
        let expected = merges
            .iter()
            .map(|merge| reference(&flow, &executions, merge))
            .collect::<Vec<_>>();
        let owners = completion(&flow, &executions, &mut merges);

        for ((merge, owners), (before, groups)) in merges.iter().zip(owners).zip(expected) {
            let wire = &merge.wire;
            assert_eq!(merge.before, before, "{name}: the `{wire}` merge's order");
            assert_eq!(owners, groups, "{name}: the `{wire}` merge's owners");
            assert!(
                !merge.before.contains(&end),
                "{name}: the `{wire}` merge waits for the implicit end"
            );
        }

        let (producing, context) = context(&flow, &executions, &merges);
        let mut pairwise = vec![Completion::default(); merges.len()];
        compare_every_pair(&producing, &context, &mut pairwise, end);
        let mut indexed = vec![Completion::default(); merges.len()];
        compare_compatible(&producing, &context, &mut indexed, end);
        assert_eq!(
            pairwise, indexed,
            "{name}: the index and the pairwise scan disagree"
        );
    }

    /// A choice with one case per execution: the count is exactly `cases`, so
    /// this walks the boundary between the two comparison strategies, and every
    /// case is one producer of the same merge.
    fn wide_choice(cases: usize) -> ItemFn {
        use std::fmt::Write as _;

        let mut source = String::from("fn valid(value: usize) -> usize {\n");
        let _ = writeln!(source, "    #[choice(\"Which case?\")]");
        for case in 0..cases {
            let _ = writeln!(source, "    #[case(\"Case {case}.\")]");
        }
        let outputs = (0..cases)
            .map(|case| format!("case_{case}"))
            .collect::<Vec<_>>()
            .join(", ");
        let arms = (0..cases)
            .map(|case| {
                if case + 1 == cases {
                    "_ => ()".to_owned()
                } else {
                    format!("{case} => ()")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            source,
            "    let ({outputs}) = |value| match value {{ {arms} }};"
        );
        for case in 0..cases {
            let _ = writeln!(
                source,
                "    #[action(\"Build {case}.\")] let built = |case_{case}| {case}usize;"
            );
        }
        source.push_str("    |built| return built;\n}\n");
        syn::parse_str(&source).expect("the generated flow parses")
    }

    #[test]
    fn the_index_agrees_with_the_pairwise_reference_over_the_fixture_corpus() {
        let corpus = kaalang_testing::corpus::corpus();
        kaalang_testing::corpus::assert_whole_tree(&corpus);
        for (name, function, _) in corpus {
            agrees(&name, &function);
        }
    }

    #[test]
    fn the_index_agrees_on_either_side_of_the_pairwise_bound() {
        for cases in [
            super::PAIRWISE_EXECUTIONS - 1,
            super::PAIRWISE_EXECUTIONS,
            super::PAIRWISE_EXECUTIONS + 1,
        ] {
            let function = wide_choice(cases);
            let (_, executions) =
                crate::analyze::walked(&function).expect("the generated flow resolves");
            assert_eq!(executions.len(), cases, "one execution per case");
            agrees(&format!("a choice of {cases} cases"), &function);
        }
    }

    #[test]
    fn the_index_agrees_on_independent_nested_and_unused_merges() {
        let shapes: [(&str, ItemFn); 4] = [
            (
                "two merges with independent contexts",
                parse_quote! {
                    fn valid(first: bool, second: bool) -> u8 {
                        #[question("Which left?")]
                        let (left_yes, left_no) = |first| { first };
                        #[action("Left yes.")] let left = |left_yes| { 1 };
                        #[action("Left no.")] let left = |left_no| { 2 };
                        #[question("Which right?")]
                        let (right_yes, right_no) = |left, second| { second };
                        #[action("Right yes.")] let right = |right_yes| { 3 };
                        #[action("Right no.")] let right = |right_no| { 4 };
                        |right| return right;
                    }
                },
            ),
            (
                "a nested selector only one branch runs",
                parse_quote! {
                    fn valid(outer: bool, inner: bool) -> u8 {
                        #[question("Go deeper?")]
                        let (deeper, direct) = |outer| { outer };
                        #[question("Which nested?")]
                        let (yes, no) = |deeper, inner| { inner };
                        #[action("Nested yes.")] let value = |yes| { 1 };
                        #[action("Nested no.")] let value = |no| { 2 };
                        #[action("Direct.")] let value = |direct| { 3 };
                        |value| return value;
                    }
                },
            ),
            (
                "one producer reached from both branches of a selector",
                parse_quote! {
                    fn valid(first: bool, second: bool) -> u8 {
                        #[question("Which outer?")]
                        let (outer_yes, outer_no) = |first| { first };
                        #[question("Which inner?")]
                        let (inner_yes, inner_no) = |outer_yes, second| { second };
                        #[action("From either inner branch.")]
                        let value = |inner_yes| { 1 };
                        #[action("Also from the inner no.")]
                        let value = |inner_no| { 2 };
                        #[action("From the outer no.")]
                        let value = |outer_no| { 3 };
                        |value| return value;
                    }
                },
            ),
            (
                "an unused output merging beside a used one",
                parse_quote! {
                    fn valid(condition: bool) -> u8 {
                        #[question("Which marker?")]
                        let (yes, no) = |condition| { condition };
                        #[action("First marker.")] let (_marker, end) = |yes| { ((), 1) };
                        #[action("Second marker.")] let (_marker, end) = |no| { ((), 2) };
                        |end| return end;
                    }
                },
            ),
        ];
        for (name, function) in shapes {
            agrees(name, &function);
        }
    }

    #[test]
    fn the_index_agrees_on_partial_merges_cycles_and_selector_chains() {
        let shapes: [(&str, ItemFn); 3] = [
            (
                "a merge one branch never reaches",
                parse_quote! {
                    fn valid(skip: bool, pick: bool) -> u8 {
                        #[question("Pick a value?")]
                        let (choose, single) = |skip| { skip };
                        #[question("Which value?")]
                        let (yes, no) = |choose, pick| { pick };
                        #[action("Yes value.")] let picked = |yes| { 1 };
                        #[action("No value.")] let picked = |no| { 2 };
                        #[action("Use the picked value.")] let value = |picked| { picked };
                        #[action("Skip picking.")] let value = |single| { 0 };
                        |value| return value;
                    }
                },
            ),
            (
                "a merge fed from inside and outside a cycle",
                parse_quote! {
                    fn valid(run: bool, done: bool, value: u8) -> u8 {
                        #[question("Run the loop?")]
                        let (enter, fallback) = |run| { run };
                        #[cycle("Wait until done.")]
                        let result = |enter, done, value| {
                            #[question("Done?")]
                            let (leave, _again) = |done| { done };
                            |leave, value| break value;
                        };
                        #[action("Use the fallback.")]
                        let result = |fallback| { 2 };
                        |result| return result;
                    }
                },
            ),
            (
                "three selectors in a row above one merge",
                parse_quote! {
                    fn valid(a: bool, b: bool, c: bool) -> bool {
                        #[question("a")]
                        let (check_b, false_result) = |a| { a };
                        #[question("b")]
                        let (check_c, false_result) = |check_b, b| { b };
                        #[question("c")]
                        let (true_result, false_result) = |check_c, c| { c };
                        #[action("Build true.")]
                        let combined = |true_result| { true };
                        #[action("Build false.")]
                        let combined = |false_result| { false };
                        #[action("Use the wider merge.")]
                        let result = |combined| { combined };
                        |result| return result;
                    }
                },
            ),
        ];
        for (name, function) in shapes {
            agrees(name, &function);
        }
    }

    /// Every execution runs a different set of selectors, which is the case the
    /// index groups worst: one group per execution and a pair of groups for
    /// every pair of them.
    #[test]
    fn the_index_agrees_when_every_execution_runs_its_own_selectors() {
        use std::fmt::Write as _;

        // One more execution than the pairwise bound, so `completion` takes
        // the index path on its own, and each of them runs a different set of
        // selectors: one group per execution, paired with every other.
        let depth = super::PAIRWISE_EXECUTIONS;
        let mut source = String::from("fn valid(seed: usize) -> usize {\n");
        for level in 0..depth {
            let input = if level == 0 {
                "seed".to_owned()
            } else {
                format!("deeper_{}", level - 1)
            };
            let _ = writeln!(source, "    #[question(\"Go past {level}?\")]");
            let _ = writeln!(
                source,
                "    let (deeper_{level}, stop_{level}) = |{input}| {input} > {level};"
            );
            let _ = writeln!(
                source,
                "    #[action(\"Stop at {level}.\")] let value = |stop_{level}| {level}usize;"
            );
        }
        let _ = writeln!(
            source,
            "    #[action(\"Run to the end.\")] let value = |deeper_{}| {depth}usize;",
            depth - 1
        );
        source.push_str("    |value| return value;\n}\n");
        let function = syn::parse_str::<ItemFn>(&source).expect("the generated flow parses");
        let (_, executions) =
            crate::analyze::walked(&function).expect("the generated flow resolves");
        assert_eq!(
            executions.len(),
            depth + 1,
            "one execution per stopping point, and one running past them all"
        );
        agrees("one selector set per execution", &function);
    }

    /// A flow whose outputs are all distinct has no merge to order. Loops reach
    /// several executions without repeating a name, which is the shape that
    /// would otherwise pay for pairs no merge ever reads: 87 of the 191 corpus
    /// flows declare no merge at all.
    #[test]
    fn a_flow_without_merges_compares_nothing() {
        let function =
            kaalang_testing::probes::flow(&kaalang_testing::probes::nested_cycles(8, 8, false));
        let (flow, executions) = crate::analyze::walked(&function).expect("the probe resolves");
        let mut merges = collect(&flow);
        assert!(merges.is_empty(), "the probe repeats no output name");
        assert!(executions.len() > 1, "the probe reaches several executions");
        assert!(
            completion(&flow, &executions, &mut merges).is_empty(),
            "a flow without merges owns no branches"
        );
    }

    /// Past the pairwise bound in executions, but only two of them produce the
    /// one merge. The strategy follows what the merges actually read, so this
    /// stays one comparison rather than an index over every execution.
    #[test]
    fn a_merge_two_executions_reach_is_compared_once() {
        use std::fmt::Write as _;

        let cases = super::PAIRWISE_EXECUTIONS;
        let mut source =
            String::from("fn valid(value: usize, pick: bool, result: usize) -> usize {\n");
        let _ = writeln!(source, "    #[choice(\"Which case?\")]");
        for case in 0..cases {
            let _ = writeln!(source, "    #[case(\"Case {case}.\")]");
        }
        let outputs = (0..cases)
            .map(|case| format!("case_{case}"))
            .collect::<Vec<_>>()
            .join(", ");
        let arms = (0..cases)
            .map(|case| {
                if case + 1 == cases {
                    "_ => ()".to_owned()
                } else {
                    format!("{case} => ()")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            source,
            "    let ({outputs}) = |value| match value {{ {arms} }};"
        );
        for case in 0..cases - 1 {
            let _ = writeln!(
                source,
                "    #[action(\"Work {case}.\")] let _done_{case} = |case_{case}| ();"
            );
        }
        let last = cases - 1;
        let _ = writeln!(
            source,
            "    #[question(\"Which value?\")] let (yes, no) = |case_{last}, pick| {{ pick }};"
        );
        let _ = writeln!(
            source,
            "    #[action(\"Yes value.\")] let picked = |yes| {{ 1usize }};"
        );
        let _ = writeln!(
            source,
            "    #[action(\"No value.\")] let picked = |no| {{ 2usize }};"
        );
        let _ = writeln!(
            source,
            "    #[action(\"Use the picked value.\")] let _used = |picked| {{ () }};"
        );
        source.push_str("    |result| return result;\n}\n");

        let function = syn::parse_str::<ItemFn>(&source).expect("the generated flow parses");
        let (flow, executions) =
            crate::analyze::walked(&function).expect("the generated flow resolves");
        assert!(
            executions.len() > super::PAIRWISE_EXECUTIONS,
            "the choice reaches past the pairwise bound"
        );
        let merges = collect(&flow);
        assert_eq!(merges.len(), 1, "only the question's branches merge");
        let (producing, _) = context(&flow, &executions, &merges);
        assert_eq!(producing.len(), 2, "only two executions produce that merge");
        agrees("a merge two executions reach", &function);
    }

    /// Two private chains under a shared prefix: the executions of one chain
    /// form one group and the other chain's another, large enough and sharing
    /// few enough selectors that the pair of groups is worth indexing. Nothing
    /// else here reaches the crossing case, where a comparison must join the
    /// two sides rather than stay inside one group.
    fn split_chains(prefix: usize, private: usize) -> ItemFn {
        use std::fmt::Write as _;

        let mut source = String::from("fn valid(seed: usize, top: bool) -> usize {\n");
        let mut wire = "seed".to_owned();
        for level in 0..prefix {
            let _ = writeln!(source, "    #[question(\"Shared {level}?\")]");
            let _ = writeln!(
                source,
                "    let (shared_yes_{level}, shared_no_{level}) = |{wire}| {wire} > {level};"
            );
            let _ = writeln!(
                source,
                "    #[action(\"Shared yes {level}.\")] let shared_{level} = |shared_yes_{level}| {level}usize;"
            );
            let _ = writeln!(
                source,
                "    #[action(\"Shared no {level}.\")] let shared_{level} = |shared_no_{level}| {level}usize + 1;"
            );
            wire = format!("shared_{level}");
        }
        let _ = writeln!(source, "    #[question(\"Which side?\")]");
        let _ = writeln!(source, "    let (left, right) = |{wire}, top| {{ top }};");
        for side in ["left", "right"] {
            let _ = writeln!(
                source,
                "    #[action(\"Enter {side}.\")] let {side}_0 = |{side}| 0usize;"
            );
            for level in 0..private {
                let _ = writeln!(source, "    #[question(\"{side} {level}?\")]");
                let _ = writeln!(
                    source,
                    "    let ({side}_yes_{level}, {side}_no_{level}) = |{side}_{level}| {side}_{level} > {level};"
                );
                let next = level + 1;
                let _ = writeln!(
                    source,
                    "    #[action(\"{side} yes {level}.\")] let {side}_{next} = |{side}_yes_{level}| {level}usize;"
                );
                let _ = writeln!(
                    source,
                    "    #[action(\"{side} no {level}.\")] let {side}_{next} = |{side}_no_{level}| {level}usize + 1;"
                );
            }
            let _ = writeln!(
                source,
                "    #[action(\"Finish {side}.\")] let value = |{side}_{private}| {side}_{private};"
            );
        }
        source.push_str("    |value| return value;\n}\n");
        syn::parse_str(&source).expect("the generated flow parses")
    }

    /// The index is only worth building where it wins, so the group pairs of a
    /// small flow take their own pairs instead and leave it unexercised. These
    /// two reach it: one group holding every execution of a flow that branches
    /// seven times, and two groups a shared prefix splits apart.
    #[test]
    fn the_index_agrees_where_it_is_the_cheaper_strategy() {
        agrees(
            "seven branching stages",
            &kaalang_testing::probes::flow(&kaalang_testing::probes::branching(7)),
        );
        agrees("two chains under a shared prefix", &split_chains(2, 4));
    }
}
