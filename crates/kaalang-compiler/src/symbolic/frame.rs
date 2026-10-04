//! The exact paired-history relations seen from each finite cycle frame.

use std::collections::BTreeMap;

use super::{
    Executions,
    condition::{ALWAYS, Condition, NEVER},
};
use crate::{BlockKind, ExecutionOutcome, Flow, ProducerId, WireMerge};

pub(super) struct Frames {
    inner: Vec<Option<usize>>,
    relations: BTreeMap<Option<usize>, Vec<Condition>>,
}

impl Frames {
    pub(super) fn of(executions: &mut Executions, flow: &Flow) -> Self {
        let inner = (0..flow.blocks.len())
            .map(|block| {
                flow.enclosing(block)
                    .find(|&header| flow.blocks[header].has_alternative_outputs())
            })
            .collect::<Vec<_>>();
        let frames =
            std::iter::once(None).chain(flow.blocks.iter().enumerate().filter_map(
                |(header, block)| block.has_alternative_outputs().then_some(Some(header)),
            ));
        let partner = executions.conditions.partner(executions.domain);
        let pairs = executions.conditions.and(executions.domain, partner);
        let mut relations = BTreeMap::new();
        for frame in frames {
            let mut compatible = vec![ALWAYS; executions.selectors.len()];
            let mut differing = vec![NEVER; executions.selectors.len()];
            for (position, &selector) in executions.selectors.clone().iter().enumerate() {
                if !crate::analyze::frame::visible(&inner, frame, selector) {
                    continue;
                }
                let mut equal = executions.same_selection(position);
                if flow.blocks[selector].kind == BlockKind::Cycle {
                    let end = flow.blocks[selector]
                        .cycle_end
                        .expect("a cycle owns a body");
                    for (inside, _) in executions
                        .selectors
                        .clone()
                        .iter()
                        .enumerate()
                        .filter(|&(_, &block)| (selector + 1..end).contains(&block))
                    {
                        let same = executions.same_selection(inside);
                        equal = executions.conditions.and(equal, same);
                    }
                }
                let other = executions.conditions.partner(executions.runs[selector]);
                let both = executions.conditions.and(executions.runs[selector], other);
                differing[position] = executions.conditions.minus(both, equal);
                compatible[position] = executions.conditions.not(differing[position]);
            }
            let compared = (0..executions.selectors.len())
                .map(|position| {
                    let mut relation = executions.conditions.and(pairs, differing[position]);
                    for (other, &agrees) in compatible.iter().enumerate() {
                        if other != position {
                            relation = executions.conditions.and(relation, agrees);
                        }
                    }
                    relation
                })
                .collect();
            relations.insert(frame, compared);
        }
        Self { inner, relations }
    }

    pub(super) fn at(&self, block: usize) -> &[Condition] {
        self.in_frame(self.inner[block])
    }
    pub(super) fn in_frame(&self, frame: Option<usize>) -> &[Condition] {
        &self.relations[&frame]
    }
    pub(super) fn visible(&self, frame: Option<usize>, block: usize) -> bool {
        crate::analyze::frame::visible(&self.inner, frame, block)
    }
    pub(super) fn frame(&self, block: usize) -> Option<usize> {
        self.inner[block]
    }
    pub(super) fn of_merge(&self, merge: &WireMerge) -> Option<usize> {
        merge.producers.iter().find_map(|producer| match producer {
            ProducerId::BlockOutput { block, .. } => Some(self.inner[*block]),
            ProducerId::FlowInput(_) => None,
        })?
    }
}

impl Executions {
    fn same_selection(&mut self, position: usize) -> Condition {
        let variable = position * 2;
        let mut equal = NEVER;
        for value in 0..self.conditions.widths[variable] {
            let first = self.conditions.selected(variable, value);
            let second = self.conditions.selected(variable + 1, value);
            let both = self.conditions.and(first, second);
            equal = self.conditions.or(equal, both);
        }
        equal
    }

    pub(super) fn reaching(&mut self, flow: &Flow, block: usize) -> Condition {
        let mut reaches = NEVER;
        for (outcome, when) in self.outcomes.clone() {
            let reachable = match outcome {
                ExecutionOutcome::Return { .. } => true,
                ExecutionOutcome::Repeat { cycle_index } => {
                    flow.repeat_reaches(cycle_index, block, |first, second| {
                        let together = self.conditions.and(self.runs[first], self.runs[second]);
                        self.has(together)
                    })
                }
            };
            if reachable {
                reaches = self.conditions.or(reaches, when);
            }
        }
        // A for cycle out of items passes its body by.
        for next in flow.required_items(block) {
            let exhausted = self.selected(next, 1);
            reaches = self.conditions.minus(reaches, exhausted);
        }
        reaches
    }

    pub(super) fn merge_reaching(&mut self, flow: &Flow, merge: &WireMerge) -> Condition {
        let mut result = NEVER;
        if merge.after.is_empty() {
            for &producer in &merge.producers {
                if let ProducerId::BlockOutput { block, .. } = producer {
                    let context = self.reaching(flow, block);
                    let repeated = self.repeats(block);
                    let context = self.conditions.minus(context, repeated);
                    result = self.conditions.or(result, context);
                }
            }
        } else {
            for &block in &merge.after {
                let context = self.reaching(flow, block);
                result = self.conditions.or(result, context);
            }
        }
        self.conditions.and(self.domain, result)
    }
}
