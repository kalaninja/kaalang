//! Records the implicit merge of every repeated output name and checks that
//! source order closes each merge's branch-local work before its consumers.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{comparison, frame::Frames};
use crate::model::{
    BlockKind, BranchSelection, Execution, ExecutionOutcome, Flow, Passes, ProducerId, WireMerge,
};

pub(super) fn flow(
    flow: &Flow,
    frames: &Frames<'_>,
    merges: Vec<WireMerge>,
    owners: Vec<Vec<(usize, Vec<usize>)>>,
    ancestry: &[BTreeSet<BranchSelection>],
) -> Result<Vec<WireMerge>> {
    let executions = frames.view(None);
    let blocks = flow.blocks.len();
    let mut successors = vec![BTreeSet::new(); blocks + merges.len()];
    let mut routes_by_owner = BTreeMap::new();
    let mut groups = (0..blocks)
        .map(|_| Vec::<super::choice::Group>::new())
        .collect::<Vec<_>>();
    for dependency in executions
        .iter()
        .flat_map(|execution| &execution.dependencies)
    {
        if let ProducerId::BlockOutput { block, .. } = dependency.producer {
            successors[block].insert(dependency.capture.block);
        }
    }
    for (merge, owners) in merges.iter().zip(owners) {
        // A merge compares the selections its producers' frame sees.
        let seen = frames.view(frames.of_merge(merge));
        for (owner, branches) in owners {
            if !matches!(
                flow.blocks[owner].kind,
                BlockKind::Choice | BlockKind::Cycle
            ) {
                continue;
            }
            // The routes producing this merge and those reaching it, each with
            // its case of the owner.
            let owner_routes = routes_by_owner
                .entry(owner)
                .or_insert_with(|| super::choice::routes(ancestry, seen, owner));
            let routes = seen
                .iter()
                .zip(owner_routes.iter())
                .filter(|(execution, _)| {
                    merge
                        .producers
                        .iter()
                        .any(|&producer| flow.produces(execution, producer))
                })
                .filter_map(|(_, &route)| route)
                .collect();
            let reaching = seen
                .iter()
                .zip(owner_routes.iter())
                .filter(|(execution, _)| reaches(frames.passes(), execution, merge))
                .filter_map(|(_, &route)| route)
                .collect();
            groups[owner].push(super::choice::Group {
                cases: branches,
                routes,
                reaching,
            });
        }
    }

    for (block, groups) in groups.iter().enumerate() {
        super::choice::validate_groups(&flow.blocks[block], groups)?;
    }
    validate_order(flow, &merges, successors)?;
    for merge in &merges {
        let executions = frames.view(frames.of_merge(merge));
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
        validate_adjacency(flow, frames.passes(), executions, merge, &producers)?;
    }
    Ok(merges)
}

/// What the comparisons found for one merge: the selectors that choose between
/// its producers, and the blocks each selector decides.
#[derive(Clone, Default)]
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
                    .map(|selection| super::frame::branch(selection.branch))
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
    frames: &Frames<'_>,
    merges: &mut [WireMerge],
) -> Vec<Vec<(usize, Vec<usize>)>> {
    if frames.is_flat() {
        return completion_at(flow, frames.view(None), merges);
    }
    // Each merge compares the selections its own frame sees.
    let mut by_frame = BTreeMap::<Option<usize>, Vec<usize>>::new();
    for (index, merge) in merges.iter().enumerate() {
        by_frame
            .entry(frames.of_merge(merge))
            .or_default()
            .push(index);
    }
    let mut owners = vec![Vec::new(); merges.len()];
    for (frame, indices) in by_frame {
        let mut subset = indices
            .iter()
            .map(|&index| merges[index].clone())
            .collect::<Vec<_>>();
        let found = completion_at(flow, frames.view(frame), &mut subset);
        for ((index, merge), found) in indices.into_iter().zip(subset).zip(found) {
            merges[index] = merge;
            owners[index] = found;
        }
    }
    owners
}

fn completion_at(
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
    comparison::compare(&producing, &context, |merge, selector, first, second| {
        found[merge].compare(
            selector,
            producing[first],
            producing[second],
            context[merge][first] != context[merge][second],
            end,
        );
    });
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

/// Whether an execution reaches the position where a merge completes.
fn reaches(passes: &Passes, execution: &Execution, merge: &WireMerge) -> bool {
    if !merge.after.is_empty() {
        return merge
            .after
            .iter()
            .any(|&block| passes.reaches(execution, block));
    }
    // Without a consumer the merge completes where its producers run. A
    // repeating cycle never hands over its own result.
    merge.producers.iter().any(|producer| match *producer {
        ProducerId::BlockOutput { block, .. } => {
            passes.reaches(execution, block)
                && execution.outcome != ExecutionOutcome::Repeat { cycle_index: block }
        }
        ProducerId::FlowInput(_) => false,
    })
}

/// Producer routes occupy one interval in authored branch order.
fn validate_adjacency(
    flow: &Flow,
    passes: &Passes,
    executions: &[Execution],
    merge: &WireMerge,
    producers: &[Option<ProducerId>],
) -> Result<()> {
    let (executions, producers): (Vec<_>, Vec<_>) = executions
        .iter()
        .zip(producers)
        .filter(|(execution, _)| reaches(passes, execution, merge))
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
        return Err(gap(flow, merge));
    }
    Ok(())
}

/// Branches reaching the merge of `merge` leave a gap between its producers.
pub(crate) fn gap(flow: &Flow, merge: &WireMerge) -> Error {
    let wire = flow.wire_name(&merge.wire);
    Error::new(
        merge.wire.span(),
        format!(
            "branches reaching the `{wire}` wire merge must be adjacent, including nested branches"
        ),
    )
}

/// Checks the combined capture and merge order before any consumer can use it
/// for lowering or drawing, and that source order already closes each merge's
/// branch-local work above the blocks capturing the merged wire. `successors`
/// holds the capture edges, with one more node per merge for its own edges.
pub(crate) fn validate_order(
    flow: &Flow,
    merges: &[WireMerge],
    mut successors: Vec<BTreeSet<usize>>,
) -> Result<()> {
    for (index, merge) in merges.iter().enumerate() {
        let node = flow.blocks.len() + index;
        for &producer in &merge.producers {
            let ProducerId::BlockOutput { block, .. } = producer else {
                unreachable!("a wire merge combines block outputs")
            };
            successors[block].insert(node);
        }
        for &block in &merge.before {
            successors[block].insert(node);
        }
        successors[node].extend(&merge.after);
    }
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
        if let Some(block) = reachable(&successors, merge_node)
            .into_iter()
            .find(|&node| successors[node].contains(&merge_node))
        {
            let wire = flow.wire_name(&merge.wire);
            let message = if flow.blocks[block].transition_target.is_some() {
                let signal = flow.wire_name(&flow.blocks[block].inputs[0].ident);
                format!(
                    "this kaalang stage transition exports `{signal}` before the `{wire}` wire merge finishes, but it waits for a value from after that merge"
                )
            } else {
                format!(
                    "this kaalang block must finish before the `{wire}` wire merge, but it waits for a value from after that merge"
                )
            };
            return Err(Error::new(flow.blocks[block].span, message));
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
        let message = if flow.blocks[block].transition_target.is_some() {
            let signal = flow.wire_name(&flow.blocks[block].inputs[0].ident);
            format!(
                "this kaalang stage transition exports `{signal}` before the `{wire}` wire merge finishes; declare the signal above the blocks that capture the merged wire"
            )
        } else {
            format!(
                "this kaalang block must finish before the `{wire}` wire merge; declare it above the blocks that capture the merged wire"
            )
        };
        return Err(Error::new(flow.blocks[block].span, message));
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
pub(crate) fn collect(flow: &Flow) -> Vec<WireMerge> {
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
            // A capture inside a cycle reaches the wire through that cycle's
            // derived input, so the cycle is the consumer at the merge's level.
            after: flow
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(block, declaration)| {
                    (declaration.parent == flow.producer_cycle(producers[0])
                        && declaration.inputs.iter().any(|input| input.ident == wire))
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

    use super::{collect, completion, completion_at};
    use crate::{
        Execution, Flow, ProducerId, WireMerge, analyze::only_difference, build,
        tests::message as error,
    };

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
    fn a_transition_waiting_on_its_own_merge_names_the_signal() {
        // Earlier stage scope checks prevent an authored flow from reaching
        // this dependency cycle, so pin the defensive diagnostic directly.
        let function: ItemFn = parse_quote! {
            fn probe(go: ()) {
                #[action("Forward the signal.")]
                |go| {};
            }
        };
        let mut flow = crate::parse::flow(&function).unwrap();
        flow.blocks[0].transition_target = Some(0);
        let merge = WireMerge {
            wire: parse_quote!(shared),
            producers: Vec::new(),
            before: vec![0],
            after: vec![0],
        };
        let successors = vec![BTreeSet::new(); flow.blocks.len() + 1];

        assert_eq!(
            super::validate_order(&flow, &[merge], successors)
                .unwrap_err()
                .to_string(),
            "this kaalang stage transition exports `go` before the `shared` wire merge finishes, but it waits for a value from after that merge"
        );
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

        assert_eq!(merge(&function, "false_result").after, [4]);
        assert_eq!(merge(&function, "combined").after, [5]);
        agrees("three selectors in a row above one merge", &function);
    }

    /// The merge after a cycle with several outputs is owned by the cycle, not
    /// by the body selections that made its choice.
    #[test]
    fn a_cycle_with_several_outputs_owns_the_merge_after_it() {
        let model = build(&crate::tests::fixture(
            include_str!("../../../kaalang/tests/cycle/behavior/alternative_outputs.rs"),
            "alternative_outputs",
        ))
        .expect("the fixture is valid");
        let flow = &model.analysis.flow;
        let mut merges = collect(flow);
        let frames = crate::analyze::frame::Frames::of(flow, &model.analysis.executions);
        let owners = completion(flow, &frames, &mut merges);
        let result = merges
            .iter()
            .position(|merge| flow.wire_name(&merge.wire) == "result")
            .expect("the outputs' continuations merge");
        assert_eq!(
            owners[result]
                .iter()
                .map(|&(owner, _)| owner)
                .collect::<Vec<_>>(),
            [0]
        );
    }

    #[test]
    fn a_repeat_route_does_not_split_a_merge_after_its_cycle() {
        let source = r#"
            fn valid(run: bool, done: bool, value: u8) -> u8 {
                #[question("Run the cycle?")]
                let (enter, fallback) = |run| { run };
                #[cycle("Wait until done.")]
                let result = |enter| loop {
                    #[question("Done?")]
                    let (leave, again) = |done| { done };
                    #[action("Keep the value.")]
                    let result = |leave, value| { value };
                    |again| continue;
                };
                #[action("Use the fallback.")]
                let result = |fallback| { 2 };
                |result| return result;
            }
        "#;
        for source in [
            source.to_owned(),
            source.replace("(leave, again)", "(again, leave)"),
        ] {
            let function = syn::parse_str::<ItemFn>(&source).expect("the flow parses");
            build(&function).expect("a repeat does not reach the merge after its cycle");
            agrees("a merge fed from inside and outside a cycle", &function);
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
        agrees("an unused output merging beside a used one", &function);
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

    /// An independent pairwise scan, kept as the completion baseline. Written
    /// apart from `record_difference`, so a mistake in either shows up as a
    /// disagreement.
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

    /// Checks that `completion` decides what the reference does, and that no
    /// merge waits for the implicit end.
    fn agrees(name: &str, function: &ItemFn) {
        let parts = if let Some(part) = crate::analyze::walked(function) {
            vec![part]
        } else {
            let analysis = crate::analyze(function).expect("the staged flow analyzes");
            let mut parts = vec![(analysis.flow, analysis.executions.to_vec())];
            parts.extend(analysis.stages.into_iter().map(|stage| {
                let local = *stage.analysis;
                (local.flow, local.executions.to_vec())
            }));
            parts
        };
        for (flow, executions) in parts {
            let end = flow.blocks.len() - 1;
            let mut merges = collect(&flow);
            let expected = merges
                .iter()
                .map(|merge| reference(&flow, &executions, merge))
                .collect::<Vec<_>>();
            let owners = completion_at(&flow, &executions, &mut merges);

            for ((merge, owners), (before, groups)) in merges.iter().zip(owners).zip(expected) {
                let wire = &merge.wire;
                assert_eq!(merge.before, before, "{name}: the `{wire}` merge's order");
                assert_eq!(owners, groups, "{name}: the `{wire}` merge's owners");
                assert!(
                    !merge.before.contains(&end),
                    "{name}: the `{wire}` merge waits for the implicit end"
                );
            }
        }
    }

    #[test]
    fn completion_agrees_with_the_reference_over_the_fixture_corpus() {
        use kaalang_testing::corpus::{self, Suite};

        let corpus = corpus::corpus(Suite::All);
        corpus::assert_corpus_shape(&corpus, Suite::All);
        for (name, function, _) in corpus {
            agrees(&name, &function);
        }
    }

    #[test]
    fn completion_agrees_on_independent_nested_and_partial_merges() {
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
        ];
        for (name, function) in shapes {
            agrees(name, &function);
        }
    }

    /// A flow whose outputs are all distinct has no merge to order. Cycles reach
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
            completion_at(&flow, &executions, &mut merges).is_empty(),
            "a flow without merges owns no branches"
        );
    }
}
