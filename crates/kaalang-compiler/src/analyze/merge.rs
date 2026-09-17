//! Records the implicit merge of every repeated output name and checks that
//! source order closes each merge's branch-local work before its consumers.

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

/// The blocks that must finish before one merge, and the branch sets of the
/// selections that choose between its producers. Production defines the context
/// even when an execution never captures the wire: an unused `_` output merges
/// like any other repeated name.
///
/// ponytail: quadratic execution-pair comparisons per merge; index executions
/// by selection or share comparisons across passes if profiling warrants it.
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
}
