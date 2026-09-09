//! Replays a lowering plan against every validated execution before using it.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    BlockKind, Branch, CaptureDependency, CaptureId, Execution, ExecutionPlan, Flow, Join,
    JoinTarget, ProducerId, WireMerge,
};

pub(super) fn plan(
    flow: &Flow,
    plan: &ExecutionPlan,
    executions: &[Execution],
    merges: &[WireMerge],
) -> bool {
    let mut bodies = vec![0; flow.blocks.len() - 1];
    count(plan, &mut bodies);
    bodies.iter().all(|&count| count == 1)
        && executions.iter().all(|execution| {
            let mut replay = Replay {
                flow,
                execution,
                merges,
                available: flow
                    .flow_inputs
                    .iter()
                    .enumerate()
                    .map(|(index, name)| (name.clone(), ProducerId::FlowInput(index)))
                    .collect(),
                ran: BTreeSet::new(),
                last: None,
                dependencies: BTreeSet::new(),
            };
            matches!(replay.walk(plan), Some(Exit::End))
                && replay
                    .ran
                    .iter()
                    .copied()
                    .eq(execution.blocks.iter().copied())
                && replay
                    .dependencies
                    .iter()
                    .copied()
                    .eq(execution.dependencies.iter().copied())
        })
}

/// Counts how many times the plan emits each computational block.
pub(crate) fn count(plan: &ExecutionPlan, bodies: &mut [usize]) {
    match plan {
        ExecutionPlan::Action { index, next } => {
            bodies[*index] += 1;
            count(next, bodies);
        }
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => {
            bodies[*index] += 1;
            for branch in branches {
                count(&branch.plan, bodies);
            }
            if let Some(join) = join {
                count(&join.next, bodies);
            }
        }
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => {
            bodies[*index] += 1;
            for branch in branches {
                count(&branch.plan, bodies);
            }
            for join in joins {
                count(&join.next, bodies);
            }
        }
        ExecutionPlan::End { body, .. } => count(body, bodies),
        ExecutionPlan::EndArrival { .. } | ExecutionPlan::Yield { .. } => {}
    }
}

enum Exit {
    End,
    Yield(JoinTarget),
}

struct Replay<'a> {
    flow: &'a Flow,
    execution: &'a Execution,
    merges: &'a [WireMerge],
    available: BTreeMap<Ident, ProducerId>,
    ran: BTreeSet<usize>,
    /// The block this execution entered last. Source order is the execution
    /// order, so `ran` alone would not catch a plan that emits the same set of
    /// blocks in another sequence.
    last: Option<usize>,
    dependencies: BTreeSet<CaptureDependency>,
}

impl Replay<'_> {
    fn capture(&mut self, block: usize) -> Option<()> {
        if !super::settled(self.merges, self.execution, block, &self.ran) {
            return None;
        }
        for (input, declaration) in self.flow.blocks[block].inputs.iter().enumerate() {
            let producer = *self.available.get(&declaration.ident)?;
            self.dependencies.insert(CaptureDependency {
                producer,
                capture: CaptureId { block, input },
            });
        }
        Some(())
    }

    fn enter(&mut self, block: usize, kind: BlockKind) -> Option<()> {
        if self.flow.blocks[block].kind != kind
            || !self.execution.participates(block)
            || self.last.is_some_and(|last| block <= last)
            || !self.ran.insert(block)
        {
            return None;
        }
        self.last = Some(block);
        self.capture(block)?;
        let selected = self.execution.selected(block);
        for (output, name) in self.flow.blocks[block].outputs.iter().enumerate() {
            if selected.is_none_or(|branch| branch == output) {
                self.available
                    .insert(name.clone(), ProducerId::BlockOutput { block, output });
            }
        }
        Some(())
    }

    fn walk(&mut self, plan: &ExecutionPlan) -> Option<Exit> {
        match plan {
            ExecutionPlan::Action { index, next } => {
                self.enter(*index, BlockKind::Action)?;
                self.walk(next)
            }
            ExecutionPlan::Question {
                index,
                branches,
                join,
            } => {
                self.enter(*index, BlockKind::Question)?;
                self.branch(*index, branches, join.as_slice())
            }
            ExecutionPlan::Choice {
                index,
                branches,
                joins,
            } => {
                self.enter(*index, BlockKind::Choice)?;
                self.branch(*index, branches, joins)
            }
            ExecutionPlan::End { body, .. } => self.walk(body),
            ExecutionPlan::EndArrival { result } => {
                let end = self.flow.blocks.len() - 1;
                if self.flow.blocks[end].inputs[0].ident != *result {
                    return None;
                }
                self.capture(end)?;
                Some(Exit::End)
            }
            ExecutionPlan::Yield { wires, join } => wires
                .iter()
                .all(|wire| self.available.contains_key(wire))
                .then_some(Exit::Yield(*join)),
        }
    }

    /// A join's continuation may hand its value to a later join of the same
    /// block, so one branch can pass through several of them in turn.
    fn branch(&mut self, block: usize, branches: &[Branch], joins: &[Join]) -> Option<Exit> {
        let selected = self.execution.selected(block)?;
        let outside = self
            .available
            .iter()
            .filter(|(_, producer)| {
                !matches!(producer, ProducerId::BlockOutput { block: source, .. } if *source == block)
            })
            .map(|(wire, _)| wire.clone())
            .collect::<BTreeSet<_>>();
        let mut exit = self.walk(&branches.get(selected)?.plan)?;
        let mut entered = None;
        while let Exit::Yield(target) = exit {
            if target.block != block || entered.is_some_and(|last| target.join <= last) {
                break;
            }
            let join = joins.get(target.join)?;
            if !join.branches.contains(&selected) {
                return None;
            }
            self.available
                .retain(|wire, _| outside.contains(wire) || join.wires.contains(wire));
            entered = Some(target.join);
            exit = self.walk(&join.next)?;
        }
        Some(exit)
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn rejects_a_plan_that_skips_an_independent_effect() {
        let model = crate::build(&parse_quote! {
            fn effects() {
                #[action("First")]
                let first = || {};
                #[action("Second")]
                let second = || {};
                #[action("Finish")]
                let result = |first, second| {};
            }
        })
        .expect("the independent effects are valid");
        assert!(plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
        let incomplete = ExecutionPlan::Action {
            index: 0,
            next: Box::new(ExecutionPlan::EndArrival {
                result: Ident::new("absent", Span::call_site()),
            }),
        };
        assert!(!plan(
            &model.flow,
            &incomplete,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_join_that_drops_a_needed_binding() {
        let mut model = crate::build(&parse_quote! {
            fn choose(flag: bool) -> u8 {
                #[question("Choose")]
                let (yes, no) = |flag| { flag };
                #[action("Yes value")]
                let value = |yes| { 1u8 };
                #[action("No value")]
                let value = |no| { 2u8 };
                #[action("Use the value")]
                let result = |value| { value };
            }
        })
        .expect("the alternative producers are valid");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Question {
            join: Some(join), ..
        } = body.as_mut()
        else {
            unreachable!()
        };
        join.wires.clear();
        assert!(!plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_plan_that_reorders_two_blocks() {
        let model = crate::build(&parse_quote! {
            fn effects() {
                #[action("First")]
                let first = || {};
                #[action("Second")]
                let second = || {};
                #[action("Finish")]
                let result = |first, second| {};
            }
        })
        .expect("the effects are valid");
        let reordered = ExecutionPlan::Action {
            index: 1,
            next: Box::new(ExecutionPlan::Action {
                index: 0,
                next: Box::new(ExecutionPlan::Action {
                    index: 2,
                    next: Box::new(ExecutionPlan::EndArrival {
                        result: Ident::new("result", Span::call_site()),
                    }),
                }),
            }),
        };
        assert!(!plan(
            &model.flow,
            &reordered,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_capture_before_branch_local_work_finishes() {
        let mut function: syn::ItemFn = parse_quote! {
            fn choose(flag: bool) -> u8 {
                #[question("Choose")]
                let (yes, no) = |flag| { flag };
                #[action("Yes value")]
                let (value, yes_work) = |yes| { (1u8, ()) };
                #[action("No value")]
                let (value, no_work) = |no| { (2u8, ()) };
                #[action("Finish the yes branch")]
                let done = |yes_work| {};
                #[action("Finish the no branch")]
                let done = |no_work| {};
                #[action("Use the merged value")]
                let used = |value| { value };
                #[action("Finish")]
                let result = |used, done| { used };
            }
        };
        let model = crate::build(&function).expect("branch-local work finishes above the capture");
        assert!(plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));

        // Capturing the merged value above the branch-local work it waits for
        // leaves that work with nowhere to run.
        function.block.stmts.swap(3, 5);
        function.block.stmts.swap(4, 5);
        assert_eq!(
            crate::tests::message(&function),
            "this kaalang block must finish before the `value` wire merge; declare it above the blocks that capture the merged wire"
        );
    }

    #[test]
    fn a_selection_deciding_a_consumer_follows_the_producer_in_source_order() {
        for (kind, selection) in [
            (
                "question",
                parse_quote! {
                    #[question("Use the setup?")]
                    let (yes, no) = |flag| { flag };
                },
            ),
            (
                "choice",
                parse_quote! {
                    #[choice("Use the setup?")]
                    #[case("Use it.")]
                    #[case("Skip it.")]
                    let (yes, no) = |flag| { match flag { true => (), false => () } };
                },
            ),
        ] {
            let mut function: syn::ItemFn = parse_quote! {
                fn choose(flag: bool, seed: u8) -> u8 {
                    #[action("Prepare the setup.")]
                    let (setup, fallback) = |seed| { (seed, 0u8) };
                    #[question("Use the setup?")]
                    let (yes, no) = |flag| { flag };
                    #[action("Use it.")]
                    let result = |yes, setup| { setup };
                    #[action("Skip it.")]
                    let result = |no, fallback| { fallback };
                }
            };
            function.block.stmts[1] = selection;
            let model = crate::build(&function).expect("the setup is prepared above the selection");
            assert_eq!(model.flow.blocks[1].inputs.len(), 1);
            for execution in &model.executions {
                assert_eq!(
                    execution.blocks,
                    [
                        0,
                        1,
                        if execution.branches[0].branch == 0 {
                            2
                        } else {
                            3
                        }
                    ]
                );
            }
            assert!(plan(
                &model.flow,
                &model.execution_plan,
                &model.executions,
                &model.merges
            ));

            // The selection opens its branches before the shared setup runs.
            function.block.stmts.swap(0, 1);
            assert_eq!(
                crate::tests::message(&function),
                crate::tests::branch_placement(kind, "Use the setup?")
            );
        }
    }

    #[test]
    fn a_merge_completes_above_the_selection_that_decides_its_consumers() {
        let mut function: syn::ItemFn = parse_quote! {
            fn choose(report: bool, flag: bool) -> u8 {
                #[question("Choose the value.")]
                let (first, second) = |flag| { flag };
                #[action("First value.")]
                let value = |first| { 1u8 };
                #[action("Second value.")]
                let value = |second| { 2u8 };
                #[question("Report the value?")]
                let (yes, no) = |report| { report };
                #[action("Report it.")]
                let result = |yes, value| { value };
                #[action("Report nothing.")]
                let result = |no, value| { 0u8 };
            }
        };
        let model = crate::build(&function).expect("the merge completes above the selection");
        let merge = model
            .merges
            .iter()
            .find(|merge| merge.wire == "value")
            .expect("the `value` wire merges");
        assert_eq!(merge.after, [4, 5]);
        let group = model
            .convergence_groups
            .iter()
            .find(|group| group.branching_block == 0)
            .expect("the first question owns a group");
        assert_eq!(group.continuation, [4, 5]);
        assert!(plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));

        // Asking first leaves the second question inside the open branches.
        function.block.stmts.swap(0, 3);
        function.block.stmts.swap(1, 3);
        function.block.stmts.swap(2, 3);
        assert_eq!(
            crate::tests::message(&function),
            crate::tests::branch_placement("question", "Report the value?")
        );
    }
}
