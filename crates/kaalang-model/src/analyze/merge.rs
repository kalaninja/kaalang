//! Places implicit wire merges after branch-local computation and before
//! every capture of the merged name, independently of the lowering plan.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, Execution, Flow, ProducerId, WireMerge};

use super::only_difference;

// ponytail: comparing every pair of producing executions per merge costs
// O(merges * executions² * blocks); reuse the participation pass or index the
// executions by selection if flows grow large enough to notice.
pub(super) fn flow(flow: &Flow, executions: &[Execution]) -> Result<Vec<WireMerge>> {
    let mut merges = collect(flow);
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
    let orderings = merges
        .iter()
        .map(|merge| ordering(flow, executions, merge))
        .collect::<Vec<_>>();
    for ((index, merge), (before, owners)) in merges.iter_mut().enumerate().zip(orderings) {
        let node = blocks + index;
        for &producer in &merge.producers {
            let ProducerId::BlockOutput { block, .. } = producer else {
                unreachable!("a wire merge combines block outputs")
            };
            successors[block].insert(node);
        }
        for (block, declaration) in flow.blocks.iter().enumerate() {
            if declaration
                .inputs
                .iter()
                .any(|input| input.ident == merge.wire)
            {
                successors[node].insert(block);
            }
        }
        for &block in &before {
            successors[block].insert(node);
        }
        for (owner, branches) in owners {
            groups[owner].entry(branches).or_default().insert(node);
        }
        merge.before = before;
    }

    for (block, groups) in groups.into_iter().enumerate() {
        super::choice::validate_groups(
            &flow.blocks[block].outputs,
            &groups.into_iter().collect::<Vec<_>>(),
        )?;
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
        if let Some(block) = cycle_block(&successors, blocks + index) {
            let wire = &merge.wire;
            return Err(Error::new(
                flow.blocks[block].span,
                format!(
                    "this kaalang block must finish before the `{wire}` wire merge, but it waits for a value from after that merge"
                ),
            ));
        }
    }
    Ok(merges)
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

/// The blocks that must finish before one merge, and the case sets of the
/// choices that select between its producers. Production defines the context
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
        .filter(|&owner| flow.blocks[owner].kind == BlockKind::Choice)
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
fn collect(flow: &Flow) -> Vec<WireMerge> {
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
            wire,
            producers,
            before: Vec::new(),
        })
        .collect::<Vec<_>>();
    merges.sort_by_key(|merge| merge.producers[0]);
    merges
}

fn produced(execution: &Execution, producer: ProducerId) -> bool {
    let ProducerId::BlockOutput { block, output } = producer else {
        return false;
    };
    execution.participates(block)
        && execution
            .branches
            .iter()
            .find(|selection| selection.block == block)
            .is_none_or(|selection| selection.branch == output)
}

/// The earliest block that both waits for `merge` and must run before it.
/// Original capture dependencies are acyclic, so every cycle contains a merge,
/// and it closes at a block: only blocks have an edge into a merge node.
/// Reporting that block names the offending statement rather than the merged
/// wire's first producer.
fn cycle_block(successors: &[BTreeSet<usize>], merge: usize) -> Option<usize> {
    let mut reached = BTreeSet::new();
    let mut pending = successors[merge].iter().copied().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        if reached.insert(node) {
            pending.extend(&successors[node]);
        }
    }
    reached
        .into_iter()
        .find(|&node| successors[node].contains(&merge))
}

#[cfg(test)]
mod tests {
    use proc_macro2::Ident;
    use syn::{ItemFn, parse_quote};

    use crate::{ProducerId, WireMerge, build};

    fn merge(function: &ItemFn, wire: &str) -> WireMerge {
        let model = build(function).expect("the flow is valid");
        model
            .merges
            .into_iter()
            .find(|merge| merge.wire == wire)
            .unwrap_or_else(|| panic!("the `{wire}` wire merges"))
    }

    fn error(function: &ItemFn) -> String {
        match build(function) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("the flow is rejected"),
        }
    }

    fn output(block: usize, output: usize) -> ProducerId {
        ProducerId::BlockOutput { block, output }
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
