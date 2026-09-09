//! Records the implicit merge of every repeated output name and checks that
//! source order closes each merge's branch-local work before its consumers.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, Execution, Flow, ProducerId, WireMerge};

use super::{only_difference, produced};

// ponytail: comparing every pair of producing executions per merge costs
// O(merges * executions² * blocks); reuse the participation pass or index the
// executions by selection if flows grow large enough to notice.
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
    validate_nesting(flow, executions, &merges, &successors)?;
    Ok(merges)
}

/// Records branch completion before placement checks common work. Validation
/// of the resulting merge order follows the execution and capture diagnostics.
pub(super) fn completion(
    flow: &Flow,
    executions: &[Execution],
    merges: &mut [WireMerge],
) -> Vec<Vec<(usize, Vec<usize>)>> {
    merges
        .iter_mut()
        .map(|merge| {
            let (before, owners) = ordering(flow, executions, merge);
            merge.before = before;
            owners
        })
        .collect()
}

/// Merge groups stay adjacent across nested selections. An outside branch may
/// leave for end, but cannot rejoin an ordinary continuation after its merge.
fn validate_nesting(
    flow: &Flow,
    executions: &[Execution],
    merges: &[WireMerge],
    successors: &[BTreeSet<usize>],
) -> Result<()> {
    let contexts = merges
        .iter()
        .map(|merge| {
            let producers = executions
                .iter()
                .map(|execution| {
                    merge
                        .producers
                        .iter()
                        .copied()
                        .find(|&producer| produced(execution, producer))
                })
                .collect::<Vec<_>>();
            validate_adjacency(executions, merge, &producers)?;
            Ok(producers
                .iter()
                .enumerate()
                .filter_map(|(index, producer)| producer.is_some().then_some(index))
                .collect::<BTreeSet<_>>())
        })
        .collect::<Result<Vec<_>>>()?;
    let result = &flow.blocks[flow.blocks.len() - 1].inputs[0].ident;
    let mut offending = None;
    for (index, context) in contexts.iter().enumerate() {
        let following = reachable(successors, flow.blocks.len() + index);
        for &run in context {
            for (skip, execution) in executions.iter().enumerate() {
                if context.contains(&skip) {
                    continue;
                }
                let Some(nested) = only_difference(&executions[run], execution) else {
                    continue;
                };
                // A partial merge entirely within this selection is valid.
                // An enclosing merge also receives a branch outside it.
                if context.iter().all(|&e| executions[e].participates(nested)) {
                    continue;
                }
                if let Some(later) = merges.iter().enumerate().position(|(later, merge)| {
                    &merge.wire != result
                        && following.contains(&(flow.blocks.len() + later))
                        && contexts[later].contains(&skip)
                }) {
                    let violation = (nested, index, later);
                    offending = Some(offending.map_or(violation, |old| violation.min(old)));
                }
            }
        }
    }
    if let Some((nested, skipped, later)) = offending {
        let skipped = &merges[skipped].wire;
        let later = &merges[later].wire;
        return Err(Error::new(
            flow.blocks[nested].span,
            format!(
                "a nested kaalang branch cannot bypass the `{skipped}` wire merge and rejoin at `{later}`; merge before or with the enclosing branches"
            ),
        ));
    }
    Ok(())
}

/// Unfold only selections that change this wire's producer or its presence.
/// Earlier independent selections and questions after the merge do not split
/// its branch interval. Source order puts each deciding ancestor before its
/// descendants, so their projected traces sort in authored branch order.
fn validate_adjacency(
    executions: &[Execution],
    merge: &WireMerge,
    producers: &[Option<ProducerId>],
) -> Result<()> {
    let mut selectors = BTreeSet::new();
    for (first, execution) in executions.iter().enumerate() {
        for (second, other) in executions.iter().enumerate().skip(first + 1) {
            if producers[first] != producers[second]
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
    let first = ordered.iter().position(|&index| producers[index].is_some());
    let last = ordered
        .iter()
        .rposition(|&index| producers[index].is_some());
    if let (Some(first), Some(last)) = (first, last)
        && ordered[first..=last]
            .iter()
            .any(|&index| producers[index].is_none())
    {
        let wire = &merge.wire;
        return Err(Error::new(
            wire.span(),
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
        if let Some(block) = cycle_block(successors, flow.blocks.len() + index) {
            let wire = &merge.wire;
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

/// The blocks that must finish before one merge, and the branch sets of the
/// selections that choose between its producers. Production defines the context
/// even when an execution never captures the wire: an unused `_` output merges
/// like any other repeated name.
fn ordering(
    flow: &Flow,
    executions: &[Execution],
    merge: &WireMerge,
) -> (Vec<usize>, Vec<(usize, Vec<usize>)>) {
    let end = flow.blocks.len() - 1;
    let context = executions
        .iter()
        .filter_map(|execution| {
            merge.producers.iter().find_map(|&producer| {
                produced(execution, producer).then_some((execution, producer))
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
                (0..end).filter(|&block| first.participates(block) != second.participates(block)),
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

/// The earliest block that both waits for `merge` and must run before it.
/// Original capture dependencies are acyclic, so every cycle contains a merge,
/// and it closes at a block: only blocks have an edge into a merge node.
/// Reporting that block names the offending statement rather than the merged
/// wire's first producer.
fn cycle_block(successors: &[BTreeSet<usize>], merge: usize) -> Option<usize> {
    reachable(successors, merge)
        .into_iter()
        .find(|&node| successors[node].contains(&merge))
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
    use proc_macro2::Ident;
    use syn::{ItemFn, parse_quote};

    use crate::tests::message as error;
    use crate::{ProducerId, WireMerge, build};

    fn merge(function: &ItemFn, wire: &str) -> WireMerge {
        let model = build(function).expect("the flow is valid");
        model
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
                |flag| -> (yes, no) { flag };
                #[action("First seed.")] |yes| -> seed { 1u8 };
                #[action("Second seed.")] |no| -> seed { 2u8 };
                #[choice("Choose the work.")]
                #[case("First.")]
                #[case("Second.")]
                #[case("Finish.")]
                |mode| -> (first, second, finish) {
                    match mode { 0 => (), 1 => (), _ => () }
                };
                #[action("First value.")] |first| -> shared { 10u8 };
                #[action("Second value.")] |second| -> shared { 20u8 };
                #[action("Finish early.")] |finish, seed| -> result { seed };
                #[action("Use the value.")] |shared, seed| -> result { shared + seed };
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
                |outer| -> (nested, direct) { outer };
                #[question("Add the marker?")]
                |nested, inner| -> (mark, skip) { inner };
                #[action("Mark the first branch.")]
                |mark| -> (_marker, result) { ((), 1u8) };
                #[action("Leave the marker absent.")]
                |skip| -> result { 2u8 };
                #[action("Mark the last branch.")]
                |direct| -> (_marker, result) { ((), 3u8) };
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
                |outer| -> (nested, direct) { outer };
                #[question("Choose the nested value.")]
                |nested, inner| -> (yes, no) { inner };
                #[action("Build the yes value.")]
                |yes| -> refined { 1u8 };
                #[action("Build the no value.")]
                |no| -> refined { 2u8 };
                #[action("Finish the nested branch.")]
                |refined| -> shared { refined + 10 };
                #[action("Build the direct value.")]
                |direct| -> shared { 3u8 };
                #[action("Use the shared value.")]
                |shared| -> result { shared };
            }
        };
        assert_eq!(merge(&function, "refined").after, [4]);
        assert_eq!(merge(&function, "shared").after, [6]);
    }

    #[test]
    fn a_branch_output_and_an_action_output_merge_before_every_capture() {
        let function: ItemFn = parse_quote! {
            fn valid(condition: bool) -> u8 {
                #[question("Which value?")]
                |condition| -> (shared, no) { condition };
                #[action("Produce the other value.")]
                |no| -> shared { () };
                #[action("Use the merged value.")]
                |shared| -> result { 0u8 };
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
                |condition| -> (shared, no) { condition };
                #[action("Produce the other value.")]
                |no| -> shared { () };
                #[action("Borrow the merged value.")]
                |&shared| -> ready { () };
                #[action("Consume it after the borrow.")]
                |shared, ready| -> result { 0u8 };
            }
        };
        assert_eq!(merge(&function, "shared").before, [1]);
    }

    #[test]
    fn unused_alternative_outputs_still_record_a_merge() {
        let function: ItemFn = parse_quote! {
            fn valid(condition: bool) -> u8 {
                #[question("Which marker?")]
                |condition| -> (yes, no) { condition };
                #[action("First marker.")]
                |yes| -> (_marker, result) { ((), 1u8) };
                #[action("Second marker.")]
                |no| -> (_marker, result) { ((), 2u8) };
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
                |value| -> (a, b, c) {
                    match value { 0 => (), 1 => (), _ => () }
                };
                #[action("First marker.")] |a| -> (_marker, result) { ((), 1u8) };
                #[action("Middle result.")] |b| -> result { 2u8 };
                #[action("Last marker.")] |c| -> (_marker, result) { ((), 3u8) };
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
                |value| -> (a, b, c) {
                    match value { 0 => (), 1 => (), _ => () }
                };
                #[action("Left marker.")] |a| -> (_left, result) { ((), 1u8) };
                #[action("Both markers.")] |b| -> (_left, _right, result) { ((), (), 2u8) };
                #[action("Right marker.")] |c| -> (_right, result) { ((), 3u8) };
            }
        };
        assert_eq!(
            error(&function),
            "kaalang choice convergence groups must be disjoint or nested"
        );
    }
}
