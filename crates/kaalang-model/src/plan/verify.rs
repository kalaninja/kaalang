//! Replays a lowering plan against every validated execution before using it.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    BlockKind, Branch, CaptureDependency, CaptureId, Execution, ExecutionOutcome, ExecutionPlan,
    Flow, Join, JoinTarget, ProducerId, WireMerge,
};

pub(super) fn plan(
    flow: &Flow,
    plan: &ExecutionPlan,
    executions: &[Execution],
    merges: &[WireMerge],
) -> bool {
    let mut bodies = vec![0; flow.blocks.len() - 1];
    for block in emitted(plan) {
        if let Some(count) = bodies.get_mut(block) {
            *count += 1;
        }
    }
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
                loop_indices: Vec::new(),
                repeats: BTreeSet::new(),
            };
            (match (replay.walk(plan), execution.outcome) {
                (Some(Exit::Return(found)), ExecutionOutcome::Return { block_index }) => {
                    found == block_index
                }
                (Some(Exit::Repeat(found)), ExecutionOutcome::Repeat { loop_index }) => {
                    found == loop_index
                }
                _ => false,
            }) && replay
                .ran
                .iter()
                .copied()
                .eq(execution.blocks.iter().copied())
                && replay
                    .dependencies
                    .iter()
                    .copied()
                    .eq(execution.dependencies.iter().copied())
                && replay
                    .repeats
                    .iter()
                    .copied()
                    .eq(execution.repeats.iter().copied())
        })
}

/// Every authored block the plan emits, in the order it emits them. The end
/// block is emitted last; every other one belongs to exactly one body.
pub(crate) fn emitted(plan: &ExecutionPlan) -> Vec<usize> {
    let mut order = Vec::new();
    super::serial_order(plan, &mut order);
    order
}

pub(super) enum Exit {
    Return(usize),
    Yield(JoinTarget),
    Repeat(usize),
    Break(usize),
}

pub(super) struct Replay<'a> {
    pub(super) flow: &'a Flow,
    pub(super) execution: &'a Execution,
    merges: &'a [WireMerge],
    pub(super) available: BTreeMap<Ident, ProducerId>,
    ran: BTreeSet<usize>,
    /// The block this execution entered last. Source order is the execution
    /// order, so `ran` alone would not catch a plan that emits the same set of
    /// blocks in another sequence.
    last: Option<usize>,
    dependencies: BTreeSet<CaptureDependency>,
    /// Only the innermost active body may be the target of a native continue.
    pub(super) loop_indices: Vec<usize>,
    repeats: BTreeSet<usize>,
}

impl Replay<'_> {
    pub(super) fn iteration(&mut self, index: usize, body: &ExecutionPlan) -> Option<Exit> {
        self.loop_indices.push(index);
        let exit = self.walk(body);
        self.loop_indices.pop();
        exit
    }

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

    pub(super) fn enter(&mut self, block: usize, kind: BlockKind) -> Option<()> {
        if self.flow.blocks[block].kind != kind
            || !self.execution.participates(block)
            || self.last.is_some_and(|last| block <= last)
            || !self.ran.insert(block)
        {
            return None;
        }
        self.last = Some(block);
        self.capture(block)?;
        if kind != BlockKind::Loop {
            self.produce(block);
        }
        Some(())
    }

    pub(super) fn produce(&mut self, block: usize) {
        for (output, name) in self.flow.blocks[block].outputs.iter().enumerate() {
            let producer = ProducerId::BlockOutput { block, output };
            if self.flow.produces(self.execution, producer) {
                self.available.insert(name.clone(), producer);
            }
        }
    }

    pub(super) fn walk(&mut self, plan: &ExecutionPlan) -> Option<Exit> {
        match plan {
            ExecutionPlan::Loop { index, body, next } => {
                super::loop_block::replay(self, *index, body, next.as_deref())
            }
            ExecutionPlan::Break { index, target } => {
                super::break_block::replay(self, *index, *target)
            }
            ExecutionPlan::Return { index } => super::return_block::replay(self, *index),
            ExecutionPlan::Repeat { index } => (self.loop_indices.last() == Some(index)
                && self.execution.repeats.contains(index)
                && self.repeats.insert(*index))
            .then_some(Exit::Repeat(*index)),
            ExecutionPlan::Action { index, next } => {
                self.enter(*index, BlockKind::Action)?;
                self.walk(next)
            }
            ExecutionPlan::Call { index, next } => {
                self.enter(*index, BlockKind::Call)?;
                self.walk(next)
            }
            ExecutionPlan::Question {
                index,
                branches,
                joins,
            } => {
                self.enter(*index, BlockKind::Question)?;
                self.branch(*index, branches, joins)
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
    use syn::parse_quote;

    use super::*;

    #[test]
    fn rejects_a_break_target_that_is_not_active() {
        let mut model = crate::build(&parse_quote! {
            fn sequential() {
                #[cycle("Leave the first cycle.")]
                || { break; };
                #[cycle("Leave the second cycle.")]
                || { break; };
                #[action("Finish.")]
                let end = || {};

                |end| return end;
            }
        })
        .expect("both loops exit");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Loop { body, .. } = body.as_mut() else {
            unreachable!()
        };
        let ExecutionPlan::Break { target, .. } = body.as_mut() else {
            unreachable!()
        };
        *target = 2;
        // Even agreement with a corrupted resolved target cannot authorize a
        // jump into a sibling loop that has not been entered.
        model.flow.blocks[1].break_target = Some(2);
        assert!(!plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_return_outcome_at_a_different_block() {
        let mut model = crate::build(&parse_quote! {
            fn returned() -> u8 {
                #[action("Build the result.")]
                let result = || 1;

                |result| return result;
            }
        })
        .expect("the flow returns its result");
        assert_eq!(
            model.executions[0].outcome,
            ExecutionOutcome::Return { block_index: 1 }
        );
        model.executions[0].outcome = ExecutionOutcome::Return { block_index: 0 };
        assert!(!plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_body_work_moved_after_a_break() {
        let mut model = crate::build(&parse_quote! {
            fn counting(mut count: usize) -> usize {
                #[cycle("Count to three.")]
                let final_count = |mut count| {
                    #[question("Finished?")]
                    let (done, again) = |count| count == 3;
                    |done, count| break count;
                    #[action("Advance.")]
                    |again, &mut count| *count += 1;
                };

                |final_count| return final_count;
            }
        })
        .expect("the body work belongs to the repeating branch");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Loop { body, next, .. } = body.as_mut() else {
            unreachable!()
        };
        let ExecutionPlan::Question { branches, .. } = body.as_mut() else {
            unreachable!()
        };
        let mut misplaced = std::mem::replace(
            &mut branches[1].plan,
            Box::new(ExecutionPlan::Repeat { index: 0 }),
        );
        let ExecutionPlan::Action { next: suffix, .. } = misplaced.as_mut() else {
            unreachable!()
        };
        *suffix = next.take().expect("the loop has a continuation");
        *next = Some(misplaced);
        // Every block is still represented exactly once, but the exiting
        // execution would now run work it should have skipped.
        let emitted = emitted(&model.execution_plan);
        assert!(
            (0..model.flow.blocks.len() - 1).all(|block| emitted
                .iter()
                .filter(|&&each| each == block)
                .count()
                == 1)
        );
        assert!(!plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_repeat_that_skips_an_enclosing_iteration_boundary() {
        let mut model = crate::build(&parse_quote! {
            fn nested(flag: bool) -> usize {
                #[cycle("Repeat the outer cycle.")]
                |flag| {
                    #[cycle("Repeat the inner cycle.")]
                    |flag| {
                        #[question("Repeat?")]
                        let (_iterate_1, leave_1) = |flag| flag;
                        |leave_1| break;
                    };
                };
            }
        })
        .expect("a trailing inner loop can exit and repeat its parent");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Loop { body, .. } = body.as_mut() else {
            unreachable!()
        };
        let ExecutionPlan::Loop { body, next, .. } = body.as_mut() else {
            unreachable!()
        };
        let ExecutionPlan::Question { branches, .. } = body.as_mut() else {
            unreachable!()
        };
        assert!(matches!(
            branches[0].plan.as_ref(),
            ExecutionPlan::Repeat { index: 1 }
        ));
        assert!(matches!(
            next.as_deref(),
            Some(ExecutionPlan::Repeat { index: 0 })
        ));
        *branches[0].plan = ExecutionPlan::Repeat { index: 0 };
        assert!(!plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn a_diverging_inner_loop_propagates_through_its_parent() {
        let model = crate::build(&parse_quote! {
            fn nested(flag: bool) -> usize {
                #[cycle("Choose whether to finish.")]
                |flag| {
                    #[question("Enter the loop?")]
                    let (iterate_2, leave_2) = |flag| flag;
                    |leave_2| break;
                    #[cycle("Repeat forever.")]
                    |iterate_2| {};
                };
                #[action("Finish.")]
                let end = || 0;

                |end| return end;
            }
        })
        .expect("the inner loop may diverge instead of reaching end");
        assert_eq!(
            model
                .executions
                .iter()
                .map(|execution| execution.outcome)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                ExecutionOutcome::Return { block_index: 5 },
                ExecutionOutcome::Repeat { loop_index: 3 }
            ])
        );
        assert!(plan(
            &model.flow,
            &model.execution_plan,
            &model.executions,
            &model.merges
        ));
    }

    #[test]
    fn rejects_a_plan_that_skips_an_independent_effect() {
        let model = crate::build(&parse_quote! {
            fn effects() {
                #[action("First")]
                let first = || {};
                #[action("Second")]
                let second = || {};
                #[action("Finish")]
                let end = |first, second| {};

                |end| return end;
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
            next: Box::new(ExecutionPlan::Return { index: 3 }),
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
                let value = |yes| { 1 };
                #[action("No value")]
                let value = |no| { 2 };
                |value| return value;
            }
        })
        .expect("the alternative producers are valid");
        let ExecutionPlan::End { body, .. } = &mut model.execution_plan else {
            unreachable!()
        };
        let ExecutionPlan::Question { joins, .. } = body.as_mut() else {
            unreachable!()
        };
        let [join] = joins.as_mut_slice() else {
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
                let end = |first, second| {};

                |end| return end;
            }
        })
        .expect("the effects are valid");
        let reordered = ExecutionPlan::Action {
            index: 1,
            next: Box::new(ExecutionPlan::Action {
                index: 0,
                next: Box::new(ExecutionPlan::Action {
                    index: 2,
                    next: Box::new(ExecutionPlan::Return { index: 3 }),
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
                let (value, yes_work) = |yes| { (1, ()) };
                #[action("No value")]
                let (value, no_work) = |no| { (2, ()) };
                #[action("Finish the yes branch")]
                let done = |yes_work| {};
                #[action("Finish the no branch")]
                let done = |no_work| {};
                #[action("Use the merged value")]
                let used = |value| { value };
                |used, done| return used;
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
                    let (setup, fallback) = |seed| { (seed, 0) };
                    #[question("Use the setup?")]
                    let (yes, no) = |flag| { flag };
                    #[action("Use the setup.")]
                    let result = |yes, setup| setup;
                    #[action("Use the fallback.")]
                    let result = |no, fallback| fallback;
                    |result| return result;
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
                        },
                        4
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
                let value = |first| { 1 };
                #[action("Second value.")]
                let value = |second| { 2 };
                #[question("Report the value?")]
                let (yes, no) = |report| { report };
                #[action("Report it.")]
                let end = |yes, value| { value };
                #[action("Report nothing.")]
                let end = |no, value| { 0 };

                |end| return end;
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
        assert_eq!(group.continuation, [4, 5, 6]);
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
