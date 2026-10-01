//! The execution checks, queried over exact conditions rather than histories.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Error, Result};

use super::frame::Frames;
use super::{
    Executions,
    condition::{ALWAYS, Condition, NEVER},
};

use crate::{BlockKind, BranchSelection, ConvergenceGroup, Flow, ProducerId, WireMerge};

impl Executions {
    pub(super) fn merged(&mut self, merge: &WireMerge) -> Condition {
        merge.producers.iter().fold(NEVER, |sum, &producer| {
            self.conditions.or(sum, self.produced(producer))
        })
    }

    pub(super) fn completion(
        &mut self,
        flow: &Flow,
        merges: &mut [WireMerge],
        frames: &Frames,
    ) -> Vec<Vec<(usize, Vec<usize>)>> {
        merges
            .iter_mut()
            .map(|merge| {
                let relations = frames.in_frame(frames.of_merge(merge));
                let present = self.merged(merge);
                let other = self.conditions.partner(present);
                let context = self.conditions.and(present, other);
                let mut different = NEVER;
                for &producer in &merge.producers {
                    let produced = self.produced(producer);
                    let other = self.conditions.partner(produced);
                    let differs = self.conditions.minus(produced, other);
                    different = self.conditions.or(different, differs);
                }
                let mut owners = Vec::new();
                let mut before = BTreeSet::new();
                for (position, &relation) in relations.iter().enumerate() {
                    let pairs = self.conditions.and(context, relation);
                    if self.conditions.and(pairs, different) == NEVER {
                        continue;
                    }
                    let owner = self.selectors[position];
                    let branches = (0..self.conditions.widths[position * 2]
                        - usize::from(flow.blocks[owner].kind != BlockKind::Loop))
                        .filter(|&branch| {
                            let selected = self.selected(owner, branch);
                            let produces = self.conditions.and(present, selected);
                            self.has(produces)
                        })
                        .collect();
                    owners.push((owner, branches));
                    for block in 0..self.runs.len() - 1 {
                        let other = self.conditions.partner(self.runs[block]);
                        let differs = self.conditions.xor(self.runs[block], other);
                        if self.conditions.and(pairs, differs) != NEVER {
                            before.insert(block);
                        }
                    }
                }
                merge.before = before.into_iter().collect();
                owners
            })
            .collect()
    }

    pub(super) fn predecessors(
        &mut self,
        flow: &Flow,
        merges: &[WireMerge],
    ) -> Vec<Vec<Condition>> {
        let count = flow.blocks.len();
        let mut preceding = vec![vec![NEVER; count]; count];
        for (block, row) in preceding.iter_mut().enumerate() {
            for header in flow.enclosing(block) {
                row[header] = self.runs[block];
            }
        }
        for (&dependency, &when) in &self.dependencies {
            if let ProducerId::BlockOutput { block, .. } = dependency.producer {
                let row = &mut preceding[dependency.capture.block];
                row[block] = self.conditions.or(row[block], when);
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
                .collect::<BTreeSet<_>>();
            for &after in &merge.after {
                for &before in &before {
                    let when = self.conditions.and(self.runs[after], self.runs[before]);
                    preceding[after][before] = self.conditions.or(preceding[after][before], when);
                }
            }
        }
        for block in 0..count {
            for before in 0..block {
                let edge = preceding[block][before];
                if edge == NEVER {
                    continue;
                }
                let (earlier, current) = preceding.split_at_mut(block);
                for (destination, &ancestor) in
                    current[0].iter_mut().zip(&earlier[before]).take(before)
                {
                    if ancestor != NEVER {
                        let inherited = self.conditions.and(edge, ancestor);
                        *destination = self.conditions.or(*destination, inherited);
                    }
                }
            }
        }
        preceding
    }

    pub(super) fn participation(
        &mut self,
        flow: &Flow,
        frames: &Frames,
        preceding: &[Vec<Condition>],
    ) -> Result<()> {
        for block in 0..flow.blocks.len() - 1 {
            let other = self.conditions.partner(self.runs[block]);
            let differs = self.conditions.xor(self.runs[block], other);
            let reaching = self.reaching(flow, block);
            let context = self.conditions.or(reaching, self.runs[block]);
            let other_context = self.conditions.partner(context);
            let context = self.conditions.and(context, other_context);
            let deciders: Vec<_> = frames
                .at(block)
                .iter()
                .enumerate()
                .filter_map(|(position, &relation)| {
                    {
                        let paired = self.conditions.and(relation, context);
                        self.conditions.and(paired, differs) != NEVER
                    }
                    .then_some(self.selectors[position])
                })
                .collect();
            for (position, &first) in deciders.iter().enumerate() {
                for &second in &deciders[position + 1..] {
                    let dependent = self
                        .conditions
                        .or(preceding[first][second], preceding[second][first]);
                    let exit_order = crate::analyze::loop_block::closed_before(flow, first, second)
                        || crate::analyze::loop_block::closed_before(flow, second, first);
                    let together = self.conditions.and(self.runs[first], self.runs[second]);
                    if !(self.has(dependent) || exit_order && self.has(together)) {
                        return Err(crate::analyze::participation::violation(flow, block));
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // Keeps carried selections and their closing junctions together.
    pub(super) fn placement(
        &mut self,
        flow: &Flow,
        merges: &[WireMerge],
        owners: &[Vec<(usize, Vec<usize>)>],
        ancestry: &[BTreeSet<BranchSelection>],
        frames: &Frames,
    ) -> Result<()> {
        let mut carried = vec![BTreeMap::<BranchSelection, Condition>::new(); flow.blocks.len()];
        for block in 0..flow.blocks.len() - 1 {
            let captures: Vec<_> = self
                .dependencies
                .iter()
                .filter(|(dependency, _)| dependency.capture.block == block)
                .map(|(&dependency, &when)| (dependency, when))
                .collect();
            carried[block] = flow.blocks[block]
                .parent
                .map(|parent| carried[parent].clone())
                .unwrap_or_default();
            for when in carried[block].values_mut() {
                *when = self.conditions.and(*when, self.runs[block]);
            }
            for (dependency, when) in captures {
                let ProducerId::BlockOutput {
                    block: source,
                    output,
                } = dependency.producer
                else {
                    continue;
                };
                let mut selections = carried[source].clone();
                if flow.blocks[source].branch_count() > 0 {
                    selections.insert(
                        BranchSelection {
                            block: source,
                            branch: output,
                        },
                        self.produced(dependency.producer),
                    );
                }
                for (selection, inherited) in selections {
                    let inherited = self.conditions.and(when, inherited);
                    let entry = carried[block].entry(selection).or_insert(NEVER);
                    *entry = self.conditions.or(*entry, inherited);
                }
            }
        }
        for block in 0..flow.blocks.len() - 1 {
            for position in 0..self.selectors.len() {
                let selector = self.selectors[position];
                if selector >= block {
                    break;
                }
                if !frames.visible(frames.frame(block), selector)
                    || crate::analyze::loop_block::closed_before(flow, selector, block)
                {
                    continue;
                }
                for branch in 0..flow.blocks[selector].outputs.len() {
                    let selection = BranchSelection {
                        block: selector,
                        branch,
                    };
                    if ancestry[block].contains(&selection) {
                        continue;
                    }
                    let selected = self.selected(selector, branch);
                    let mut offending = self.conditions.and(self.runs[block], selected);
                    offending = self.conditions.and(offending, self.selected_runs[selector]);
                    for (merge, owners) in merges.iter().zip(owners) {
                        let position = merge
                            .producers
                            .iter()
                            .filter_map(|producer| match producer {
                                ProducerId::BlockOutput { block, .. } => Some(*block),
                                ProducerId::FlowInput(_) => None,
                            })
                            .max()
                            .unwrap_or(0);
                        if position >= block {
                            continue;
                        }
                        let mut closes = NEVER;
                        let owns = owners.iter().any(|&(owner, _)| owner == selector);
                        for &producer in &merge.producers {
                            let ProducerId::BlockOutput {
                                block: source,
                                output,
                            } = producer
                            else {
                                continue;
                            };
                            let occurs = |source: usize, output: usize| {
                                ancestry[source].contains(&selection)
                                    || (source == selector && output == branch)
                            };
                            let reached = if occurs(source, output) {
                                ALWAYS
                            } else if selector < source {
                                carried[source].get(&selection).copied().unwrap_or(NEVER)
                            } else {
                                NEVER
                            };
                            let another = merge.producers.iter().any(|producer| match *producer {
                                ProducerId::BlockOutput { block, output } => !occurs(block, output),
                                ProducerId::FlowInput(_) => false,
                            });
                            if !owns && !another {
                                continue;
                            }
                            let closes_here = if owns {
                                self.produced(producer)
                            } else {
                                self.conditions.and(self.produced(producer), reached)
                            };
                            closes = self.conditions.or(closes, closes_here);
                        }
                        let provided = self.merged(merge);
                        let shared = self.conditions.and(self.runs[block], self.runs[selector]);
                        let outside = self.conditions.minus(shared, provided);
                        if self.has(outside) {
                            continue;
                        }
                        if !merge.after.contains(&block) {
                            for &before in merge.before.iter().filter(|&&before| before >= block) {
                                closes = self.conditions.minus(closes, self.runs[before]);
                            }
                        }
                        offending = self.conditions.minus(offending, closes);
                    }
                    if self.has(offending) {
                        return Err(crate::analyze::placement::violation(flow, block, selector));
                    }
                }
            }
        }
        Ok(())
    }

    /// A choice route retains only its own selection and nested selections.
    fn routes(
        &mut self,
        flow: &Flow,
        ancestry: &[BTreeSet<BranchSelection>],
        owner: usize,
        context: Condition,
        frames: &Frames,
        frame: Option<usize>,
    ) -> Condition {
        let mut routes = NEVER;
        for branch in 0..flow.blocks[owner].outputs.len() {
            let selection = BranchSelection {
                block: owner,
                branch,
            };
            let selected = self.selected(owner, branch);
            let selected = self.conditions.and(selected, context);
            let selected = self.conditions.and(selected, self.domain);
            let visible: Vec<_> = self
                .selectors
                .iter()
                .enumerate()
                .filter_map(|(position, &block)| {
                    (frames.visible(frame, block)
                        && (block == owner || ancestry[block].contains(&selection)))
                    .then_some(position)
                })
                .collect();
            // A visible cycle selection also identifies its entire body route.
            let retained: Vec<_> = self
                .selectors
                .iter()
                .enumerate()
                .filter_map(|(position, &block)| {
                    (visible.contains(&position)
                        || visible.iter().any(|&outer| {
                            let header = self.selectors[outer];
                            flow.blocks[header]
                                .loop_end
                                .is_some_and(|end| (header + 1..end).contains(&block))
                        }))
                    .then_some(position * 2)
                })
                .collect();
            let mut route = self.conditions.project(selected, &retained);
            for position in 0..self.selectors.len() {
                let variable = position * 2;
                if !retained.contains(&variable) {
                    let inactive = self
                        .conditions
                        .selected(variable, self.conditions.widths[variable] - 1);
                    route = self.conditions.and(route, inactive);
                }
            }
            routes = self.conditions.or(routes, route);
        }
        routes
    }

    fn groups(
        &mut self,
        flow: &Flow,
        ancestry: &[BTreeSet<BranchSelection>],
        owner: usize,
        groups: &[(Vec<usize>, Condition, Condition)],
        frames: &Frames,
        frame: Option<usize>,
    ) -> Result<()> {
        let recorded: Vec<_> = groups
            .iter()
            .map(|(_, context, _)| self.routes(flow, ancestry, owner, *context, frames, frame))
            .collect();
        let reaching: Vec<_> = groups
            .iter()
            .map(|(_, _, context)| self.routes(flow, ancestry, owner, *context, frames, frame))
            .collect();
        let (noun, members) = if flow.blocks[owner].kind == BlockKind::Loop {
            ("cycle", "outputs")
        } else {
            ("choice", "branches")
        };
        let mut offending: Option<(usize, String)> = None;
        let mut report = |case, message| {
            if offending.as_ref().is_none_or(|(old, _)| case < *old) {
                offending = Some((case, message));
            }
        };
        for (position, (branches, _, _)) in groups.iter().enumerate() {
            if let (Some(&first), Some(&last)) = (branches.first(), branches.last()) {
                for case in first..=last {
                    if branches.contains(&case) {
                        continue;
                    }
                    let selected = self.selected(owner, case);
                    if self.conditions.and(reaching[position], selected) != NEVER {
                        report(
                            case,
                            format!(
                                "{members} in a kaalang {noun} convergence group must be adjacent"
                            ),
                        );
                        break;
                    }
                }
            }
            let routes = recorded[position];
            for (other_position, &other) in recorded.iter().enumerate().skip(position + 1) {
                let shared = self.conditions.and(routes, other);
                let apart = self.conditions.minus(routes, other);
                let apart = self.conditions.and(apart, reaching[other_position]);
                let other_apart = self.conditions.minus(other, routes);
                let other_apart = self.conditions.and(other_apart, reaching[position]);
                if shared == NEVER || apart == NEVER || other_apart == NEVER {
                    continue;
                }
                let assignment = self
                    .conditions
                    .witness(shared)
                    .expect("shared routes have a witness");
                let variable = self
                    .selectors
                    .binary_search(&owner)
                    .expect("a choice is a selector")
                    * 2;
                report(
                    assignment[variable],
                    format!("kaalang {noun} convergence groups must be disjoint or nested"),
                );
            }
        }
        if let Some((case, message)) = offending {
            return Err(Error::new(flow.blocks[owner].outputs[case].span(), message));
        }
        Ok(())
    }

    pub(super) fn validate_merges(
        &mut self,
        flow: &Flow,
        merges: &[WireMerge],
        owners: &[Vec<(usize, Vec<usize>)>],
        ancestry: &[BTreeSet<BranchSelection>],
        frames: &Frames,
    ) -> Result<()> {
        let mut successors = vec![BTreeSet::new(); flow.blocks.len() + merges.len()];
        for (&dependency, &when) in &self.dependencies.clone() {
            if self.has(when)
                && let ProducerId::BlockOutput { block, .. } = dependency.producer
            {
                successors[block].insert(dependency.capture.block);
            }
        }
        let mut groups = vec![Vec::new(); flow.blocks.len()];
        for ((index, merge), owners) in merges.iter().enumerate().zip(owners) {
            let node = flow.blocks.len() + index;
            for &producer in &merge.producers {
                if let ProducerId::BlockOutput { block, .. } = producer {
                    successors[block].insert(node);
                }
            }
            for &before in &merge.before {
                successors[before].insert(node);
            }
            successors[node].extend(&merge.after);
            let provided = self.merged(merge);
            for (owner, branches) in owners {
                if matches!(
                    flow.blocks[*owner].kind,
                    BlockKind::Choice | BlockKind::Loop
                ) {
                    let reaching = self.merge_reaching(flow, merge);
                    groups[*owner].push((branches.clone(), provided, reaching));
                }
            }
        }
        for (owner, groups) in groups.iter().enumerate() {
            if matches!(flow.blocks[owner].kind, BlockKind::Choice | BlockKind::Loop) {
                self.groups(flow, ancestry, owner, groups, frames, frames.frame(owner))?;
            }
        }
        crate::analyze::merge::validate_order(flow, merges, &successors)?;
        for merge in merges {
            let present = self.merged(merge);
            let context = self.merge_reaching(flow, merge);
            let absent = self.conditions.minus(context, present);
            let mut outcomes = vec![absent];
            for &producer in &merge.producers {
                let when = self.conditions.and(context, self.produced(producer));
                outcomes.push(when);
            }
            let relations = frames.in_frame(frames.of_merge(merge));
            let selectors = self.outcome_selectors(&outcomes, relations, context);
            if self.ordered(&outcomes, &selectors).gap {
                let wire = flow.wire_name(&merge.wire);
                return Err(Error::new(
                    merge.wire.span(),
                    format!(
                        "branches reaching the `{wire}` wire merge must be adjacent, including nested branches"
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn outcome_selectors(
        &mut self,
        outcomes: &[Condition],
        relations: &[Condition],
        context: Condition,
    ) -> Vec<usize> {
        let other = self.conditions.partner(context);
        let context = self.conditions.and(context, other);
        let mut different = NEVER;
        for &when in outcomes {
            let other = self.conditions.partner(when);
            let differs = self.conditions.minus(when, other);
            different = self.conditions.or(different, differs);
        }
        relations
            .iter()
            .enumerate()
            .filter_map(|(position, &relation)| {
                let pairs = self.conditions.and(relation, context);
                (self.conditions.and(pairs, different) != NEVER).then_some(position)
            })
            .collect()
    }

    pub(super) fn convergence(
        &mut self,
        flow: &Flow,
        preceding: &[Vec<Condition>],
        ancestry: &[BTreeSet<BranchSelection>],
        frames: &Frames,
    ) -> Result<Vec<ConvergenceGroup>> {
        let mut recorded = Vec::new();
        for &owner in &self.selectors.clone() {
            let mut groups = BTreeMap::<Vec<usize>, Vec<usize>>::new();
            let mut continued = Vec::new();
            for (block, row) in preceding.iter().enumerate().take(flow.blocks.len() - 1) {
                if flow.blocks[owner].loop_end.is_some_and(|end| block < end)
                    || !frames.visible(frames.frame(block), owner)
                {
                    continue;
                }
                let context = self.conditions.and(row[owner], self.runs[owner]);
                let branches: Vec<_> = (0..flow.blocks[owner].outputs.len())
                    .filter(|&branch| {
                        let selected = self.selected(owner, branch);
                        let selected = self.conditions.and(context, selected);
                        self.has(selected)
                    })
                    .collect();
                if branches.len() >= 2 {
                    groups.entry(branches.clone()).or_default().push(block);
                    let reaching = self.reaching(flow, block);
                    continued.push((branches, context, reaching));
                }
            }
            if matches!(flow.blocks[owner].kind, BlockKind::Choice | BlockKind::Loop) {
                self.groups(
                    flow,
                    ancestry,
                    owner,
                    &continued,
                    frames,
                    frames.frame(owner),
                )?;
            }
            recorded.extend(
                groups
                    .into_iter()
                    .map(|(branches, continuation)| ConvergenceGroup {
                        branching_block: owner,
                        branches,
                        continuation,
                    }),
            );
        }
        Ok(recorded)
    }
}
