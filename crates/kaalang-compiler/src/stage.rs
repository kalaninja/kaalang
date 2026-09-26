//! Resolves stage interfaces and checks each local sequence and their finite graph.

use std::collections::{BTreeSet, VecDeque};

use proc_macro2::Ident;
use syn::{Error, FnArg, ItemFn, Pat, Result};

use crate::model::{Analysis, Execution, ExecutionOutcome, Flow, ProducerId, StageAnalysis};
use crate::parse::ParsedStaged;
use crate::{analyze as local_analysis, plan, resolve, scope};

fn local(function: &ItemFn, flow: Flow, check_usage: bool) -> Result<Analysis> {
    let (executions, convergence_groups, merges) = if check_usage {
        local_analysis::flow(&flow)?
    } else {
        local_analysis::flow_without_usage(&flow)?
    };
    let execution_plan = plan::flow(&flow, &executions, &merges);
    Ok(Analysis {
        name: function.sig.ident.clone(),
        parameters: function.sig.inputs.iter().cloned().collect(),
        return_type: function.sig.output.clone(),
        flow,
        execution_plan,
        executions,
        convergence_groups,
        merges,
        stages: Vec::new(),
        common_wires: Vec::new(),
    })
}

fn available(flow: &Flow, route: &Execution, name: &Ident) -> bool {
    flow.flow_inputs.contains(name)
        || flow.blocks.iter().enumerate().any(|(block, declaration)| {
            declaration.parent.is_none()
                && declaration
                    .outputs
                    .iter()
                    .enumerate()
                    .any(|(output, wire)| {
                        wire == name
                            && flow
                                .produces(route, crate::ProducerId::BlockOutput { block, output })
                    })
        })
}

fn targets(analysis: &Analysis) -> impl Iterator<Item = usize> + '_ {
    analysis.executions.iter().filter_map(|route| {
        let ExecutionOutcome::Return { block_index } = route.outcome else {
            return None;
        };
        analysis.flow.blocks[block_index].transition_target
    })
}

fn mutable_outer(function: &ItemFn, preparation: &Flow, name: &Ident) -> bool {
    match preparation.producer(name) {
        Some(ProducerId::BlockOutput { block, output }) => {
            preparation.blocks[block].output_binding(output).mutability.is_some()
        }
        Some(ProducerId::FlowInput(index)) => match function.sig.inputs.iter().filter(|argument| {
            !matches!(argument, FnArg::Typed(argument) if matches!(argument.pat.as_ref(), Pat::Wild(_)))
        }).nth(index).expect("each flow input comes from a named parameter") {
            FnArg::Typed(argument) => matches!(argument.pat.as_ref(), Pat::Ident(binding) if binding.mutability.is_some()),
            FnArg::Receiver(receiver) => crate::parse::receiver_capture(receiver).2,
        },
        None => false,
    }
}

#[allow(clippy::too_many_lines)] // Coordinates local analyses with the shared preparation boundary.
pub(crate) fn analyze(function: &ItemFn, mut parsed: ParsedStaged) -> Result<Analysis> {
    scope::resolve(&mut parsed.preparation)?;
    resolve::flow(&parsed.preparation)?;
    let preliminary = local(function, parsed.preparation, false)?;
    let mut preparation = preliminary.flow;
    let completed = preliminary
        .executions
        .iter()
        .filter(|route| matches!(route.outcome, ExecutionOutcome::Return { .. }))
        .collect::<Vec<_>>();
    let candidates = preparation
        .flow_inputs
        .iter()
        .cloned()
        .chain(
            preparation
                .blocks
                .iter()
                .filter(|block| block.parent.is_none())
                .flat_map(|block| block.outputs.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let entry_names = parsed
        .stages
        .iter()
        .map(|stage| stage.entry.clone())
        .collect::<BTreeSet<_>>();
    let common = candidates
        .iter()
        .filter(|name| {
            !entry_names.contains(*name)
                && completed
                    .iter()
                    .all(|route| available(&preparation, route, name))
        })
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut stages = Vec::new();
    let mut stage_spans = Vec::new();
    let mut used_common = BTreeSet::new();
    for mut stage in parsed.stages.drain(..) {
        stage
            .flow
            .flow_inputs
            .retain(|name| name == &stage.entry || common.contains(name));
        scope::resolve(&mut stage.flow)?;
        for input in stage.flow.blocks.iter().flat_map(|block| &block.inputs) {
            if candidates.contains(&input.ident)
                && !entry_names.contains(&input.ident)
                && !common.contains(&input.ident)
                && stage.flow.producer(&input.ident).is_none()
            {
                return Err(Error::new(
                    input.alias.span(),
                    "a common outer wire used by a kaalang stage must be available on every preparation route",
                ));
            }
            if input.borrowed
                && input.mutable
                && common.contains(&input.ident)
                && !mutable_outer(function, &preparation, &input.ident)
            {
                return Err(Error::new(
                    input.alias.span(),
                    "a mutable capture requires the outer wire to be declared with `mut`",
                ));
            }
        }
        resolve::flow(&stage.flow)?;
        used_common.extend(
            stage
                .flow
                .blocks
                .iter()
                .flat_map(|block| &block.inputs)
                .map(|input| &input.ident)
                .filter(|name| common.contains(*name))
                .cloned(),
        );
        stage_spans.push(stage.span);
        let analysis = local(function, stage.flow, true)?;
        stages.push(StageAnalysis {
            description: stage.description,
            entry: stage.entry,
            entry_alias: stage.entry_alias,
            outputs: stage.outputs,
            analysis: Box::new(analysis),
        });
    }

    for block in preparation
        .blocks
        .iter_mut()
        .filter(|block| block.transition_target.is_some())
    {
        for wire in &used_common {
            block.inputs.push(crate::Input {
                borrowed: false,
                mutable: false,
                ident: wire.clone(),
                alias: wire.clone(),
                derived: true,
            });
        }
    }
    resolve::flow(&preparation)?;
    let mut preparation = local(function, preparation, true)?;
    let mut reachable = vec![false; stages.len()];
    let mut queue = targets(&preparation).collect::<VecDeque<_>>();
    while let Some(stage) = queue.pop_front() {
        if reachable[stage] {
            continue;
        }
        reachable[stage] = true;
        queue.extend(targets(&stages[stage].analysis));
    }
    if let Some((index, _)) = reachable.iter().enumerate().find(|(_, reached)| !**reached) {
        return Err(Error::new(
            stage_spans[index],
            "this kaalang stage is unreachable from preparation",
        ));
    }
    preparation.stages = stages;
    preparation.common_wires = used_common.into_iter().collect();
    Ok(preparation)
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    #[test]
    fn resolves_a_self_transition_and_terminal_stage() {
        let function: ItemFn = parse_quote! {
            fn count_to(limit: usize) -> usize {
                #[action("Initialize the counter.")]
                let mut counter = || 0usize;

                #[action("Begin counting.")]
                let count = || ();

                #[stage("Count to the limit.")]
                let (count, finish) = |count| {
                    #[question("Done?")]
                    let (finish, again) = |counter, limit| counter >= limit;

                    #[action("Increment and repeat.")]
                    let count = |again, &mut counter| { *counter += 1; };
                };

                #[stage("Return the count.")]
                |finish| {
                    |counter| return counter;
                };
            }
        };
        let analysis = crate::analyze(&function).expect("staged flow analyzes");
        assert_eq!(analysis.stages.len(), 2);
        assert_eq!(analysis.stages[0].entry, "count");
        assert_eq!(analysis.stages[1].entry, "finish");
        crate::expand(function).expect("the staged flow lowers");
    }

    #[test]
    fn rejects_invalid_stage_interfaces_and_routes() {
        let cases = [
            (
                "async fn f(go: ()) { #[stage(\"Finish.\")] |go| { return; }; }",
                "does not support async flows",
            ),
            (
                "fn f() { #[stage(\"Finish.\")] |finish| { return; }; }",
                "without selecting a stage signal",
            ),
            (
                "fn f() { #[action(\"A.\")] let go = || (); #[action(\"B.\")] let stop = || (); #[stage(\"Go.\")] let go = |go| { #[action(\"Again.\")] let go = |go| go; }; #[stage(\"Stop.\")] |stop| { return; }; }",
                "more than one stage signal",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] |go| { return; }; #[stage(\"Lost.\")] let lost = |lost| { #[action(\"Again.\")] let lost = |lost| lost; }; }",
                "unreachable",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] let nowhere = |go| { #[action(\"Nowhere.\")] let nowhere = || (); }; }",
                "no matching stage entry",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] let go = |go| { }; }",
                "needs a producer",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] |go| { #[action(\"Again.\")] let local = || (); }; }",
                "without a transition or `return`",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] |go| { #[action(\"Bad.\")] let go = || (); return; }; }",
                "shadow a visible outer wire",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] |go| { |&mut go| return; }; }",
                "cannot be captured through `&mut`",
            ),
            (
                "fn f() { #[action(\"Outer.\")] let shared = || 1; #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] |go| { |&mut shared| return; }; }",
                "outer wire to be declared with `mut`",
            ),
            (
                "fn f(choice: bool) { #[question(\"Choose.\")] let (yes, no) = |choice| choice; #[action(\"Left data.\")] let shared = |yes| 1; #[action(\"Left signal.\")] let go = |yes| (); #[action(\"Right signal.\")] let go = |no| (); #[stage(\"Go.\")] |go| { |shared| return; }; }",
                "available on every preparation route",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] let mut go = |go| { #[action(\"Again.\")] let go = || (); }; }",
                "cannot declare `mut`",
            ),
            (
                "fn f() { #[action(\"Go.\")] let go = || (); #[stage(\"Go.\")] let go = |go| { return; }; }",
                "declares no transition outputs",
            ),
        ];
        for (source, expected) in cases {
            let function: ItemFn = syn::parse_str(source).expect("valid Rust syntax");
            let error = crate::build(&function).err().expect("the flow is rejected");
            assert!(error.to_string().contains(expected), "{source}: {error}");
        }
    }

    #[test]
    fn accepts_a_stage_that_diverges_in_a_nested_cycle() {
        let function: ItemFn = parse_quote! {
            fn diverge() {
                #[action("Enter the only stage.")]
                let go = || ();

                #[stage("Repeat forever.")]
                |go| {
                    #[cycle("Stay in this stage.")]
                    {
                        continue;
                    };
                };
            }
        };
        let model = crate::build(&function).expect("the divergent stage has a diagram");
        assert_eq!(model.stages.len(), 1);
        assert!(
            model.stages[0]
                .topology
                .nodes
                .iter()
                .all(|node| node.kind != crate::topology::NodeKind::End)
        );
        crate::expand(function).expect("the dispatcher can diverge");
    }
}
