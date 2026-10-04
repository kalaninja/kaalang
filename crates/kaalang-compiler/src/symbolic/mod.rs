//! Exact execution conditions for finite local flow histories.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Error, Result, ext::IdentExt};

use self::condition::{Condition, Conditions, NEVER};
pub(crate) use self::walk::flow as histories;
use crate::{
    BranchSelection, CaptureDependency, CaptureId, ConvergenceGroup, Execution, ExecutionOutcome,
    Flow, FlowKind, ProducerId, WireMerge,
};

mod condition;
mod cycle;
mod frame;
mod order;
mod plan;
mod projection;
mod stage;
mod validate;
mod walk;

#[cfg(test)]
mod tests;

/// Larger finite domains use execution conditions instead of enumeration.
/// Measured on 2026-10-02 over macro expansion in the dev profile: enumeration
/// is about 15% faster up to 40 histories, the two are within 12% from 48 to
/// 56, and conditions are 20-40% faster from 64 on.
pub(crate) const ENUMERATED_HISTORY_LIMIT: usize = 48;

#[derive(Clone)]
pub(crate) struct Executions {
    conditions: Conditions,
    domain: Condition,
    runs: Vec<Condition>,
    selected_runs: Vec<Condition>,
    produced: BTreeMap<ProducerId, Condition>,
    dependencies: BTreeMap<CaptureDependency, Condition>,
    outcomes: BTreeMap<ExecutionOutcome, Condition>,
    selectors: Vec<usize>,
}

pub(crate) fn worth_factoring(flow: &Flow) -> bool {
    flow.blocks
        .iter()
        .filter(|block| block.branch_count() > 0)
        .fold(1usize, |count, block| {
            count
                .saturating_mul(block.outputs.len())
                .min(ENUMERATED_HISTORY_LIMIT + 1)
        })
        > ENUMERATED_HISTORY_LIMIT
}

#[cfg(test)]
pub(crate) fn analyze(
    flow: &Flow,
    check_usage: bool,
) -> Result<(Executions, Vec<ConvergenceGroup>, Vec<WireMerge>)> {
    validate(flow, check_usage, histories(flow)?)
}

pub(crate) fn validate(
    flow: &Flow,
    check_usage: bool,
    mut executions: Executions,
) -> Result<(Executions, Vec<ConvergenceGroup>, Vec<WireMerge>)> {
    let incomplete = executions.has(executions.runs[flow.blocks.len() - 1]);
    let completing = executions
        .outcomes
        .values()
        .fold(NEVER, |sum, &when| executions.conditions.or(sum, when));
    executions.domain = executions.conditions.and(executions.domain, completing);
    let ancestry = crate::analyze::placement::ancestry(flow);
    let mut merges = crate::analyze::merge::collect(flow);
    let frames = frame::Frames::of(&mut executions, flow);
    let owners = executions.completion(flow, &mut merges, &frames);
    executions.placement(flow, &merges, &owners, &ancestry, &frames)?;
    if incomplete {
        return Err(crate::analyze::end::missing_return(flow));
    }
    for index in 0..flow.blocks.len() - 1 {
        if !(executions.has(executions.runs[index]) || flow.may_never_run(index)) {
            return Err(crate::analyze::unreachable(flow, index));
        }
    }
    if check_usage {
        executions.captured(flow)?;
    }
    executions.branch_outputs(flow, &merges)?;
    let captures = executions.predecessors(flow, &[]);
    executions.participation(flow, &frames, &captures)?;
    executions.validate_merges(flow, &merges, &owners, &ancestry, &frames)?;
    cycle::output_order(&mut executions, flow, &frames)?;
    let precedence = executions.predecessors(flow, &merges);
    let groups = executions.convergence(flow, &precedence, &ancestry, &frames)?;
    Ok((executions, groups, merges))
}

impl Executions {
    pub(crate) fn compact(&mut self) {
        let mut roots: Vec<_> = std::iter::once(&self.domain)
            .chain(&self.runs)
            .chain(&self.selected_runs)
            .chain(self.produced.values())
            .chain(self.dependencies.values())
            .chain(self.outcomes.values())
            .copied()
            .collect();
        self.conditions.retain(&mut roots);
        let mut roots = roots.into_iter();
        self.domain = roots.next().expect("the domain was retained");
        for condition in self
            .runs
            .iter_mut()
            .chain(self.selected_runs.iter_mut())
            .chain(self.produced.values_mut())
            .chain(self.dependencies.values_mut())
            .chain(self.outcomes.values_mut())
        {
            *condition = roots.next().expect("every stored predicate was retained");
        }
    }

    fn has(&mut self, condition: Condition) -> bool {
        self.conditions.and(self.domain, condition) != NEVER
    }

    fn produced(&self, producer: ProducerId) -> Condition {
        self.produced.get(&producer).copied().unwrap_or(NEVER)
    }

    fn selected(&mut self, block: usize, branch: usize) -> Condition {
        let variable = self
            .selectors
            .binary_search(&block)
            .expect("a selector has a variable")
            * 2;
        self.conditions.selected(variable, branch)
    }

    fn expression(&self, assignment: &[usize]) -> Execution {
        Execution {
            branches: self
                .selectors
                .iter()
                .enumerate()
                .filter_map(|(position, &block)| {
                    self.conditions
                        .matches(self.selected_runs[block], assignment)
                        .then_some(BranchSelection {
                            block,
                            branch: assignment[position * 2],
                        })
                })
                .collect(),
            blocks: self.runs[..self.runs.len() - 1]
                .iter()
                .enumerate()
                .filter_map(|(block, &when)| {
                    self.conditions.matches(when, assignment).then_some(block)
                })
                .collect(),
            dependencies: self
                .dependencies
                .iter()
                .filter_map(|(&dependency, &when)| {
                    self.conditions
                        .matches(when, assignment)
                        .then_some(dependency)
                })
                .collect(),
            outcome: self
                .outcomes
                .iter()
                .find_map(|(&outcome, &when)| {
                    self.conditions.matches(when, assignment).then_some(outcome)
                })
                .expect("a validated execution returns"),
        }
    }

    pub(crate) fn exceeds(&self, limit: usize) -> bool {
        self.conditions.count_up_to(self.domain, Some(limit + 1)) > limit
    }

    pub(crate) fn len(&self) -> usize {
        self.conditions.count(self.domain)
    }

    pub(crate) fn enumerate(&self) -> Vec<Execution> {
        fn visit(
            executions: &mut Executions,
            domain: Condition,
            variable: usize,
            assignment: &mut [usize],
            result: &mut BTreeSet<Execution>,
        ) {
            if domain == NEVER {
                return;
            }
            if variable == assignment.len() {
                result.insert(executions.expression(assignment));
                return;
            }
            for branch in 0..executions.conditions.widths[variable] {
                assignment[variable] = branch;
                let restricted = executions.conditions.restrict(domain, variable, branch);
                visit(executions, restricted, variable + 2, assignment, result);
            }
        }
        let mut copy = self.clone();
        let mut result = BTreeSet::new();
        let mut assignment = vec![0; self.conditions.widths.len()];
        visit(&mut copy, self.domain, 0, &mut assignment, &mut result);
        result.into_iter().collect()
    }

    fn captured(&mut self, flow: &Flow) -> Result<()> {
        let mut consumed = BTreeMap::new();
        for (&dependency, &when) in &self.dependencies {
            let entry = consumed.entry(dependency.producer).or_insert(NEVER);
            *entry = self.conditions.or(*entry, when);
        }
        for (index, input) in flow.flow_inputs.iter().enumerate() {
            if !matches!(flow.kind, FlowKind::Stage { .. })
                && !input.unraw().to_string().starts_with('_')
                && !self.has(
                    consumed
                        .get(&ProducerId::FlowInput(index))
                        .copied()
                        .unwrap_or(NEVER),
                )
            {
                return Err(Error::new(
                    input.span(),
                    "every kaalang flow input must have a consumer",
                ));
            }
        }
        for (block, declaration) in flow.blocks.iter().enumerate() {
            for (output, name) in declaration.outputs.iter().enumerate() {
                if !declaration
                    .output_binding(output)
                    .ident
                    .unraw()
                    .to_string()
                    .starts_with('_')
                    && !self.has(
                        consumed
                            .get(&ProducerId::BlockOutput { block, output })
                            .copied()
                            .unwrap_or(NEVER),
                    )
                {
                    return Err(Error::new(
                        name.span(),
                        format!(
                            "every kaalang {} output must have a consumer",
                            crate::parse::noun(declaration.kind)
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    fn branch_outputs(&mut self, flow: &Flow, merges: &[WireMerge]) -> Result<()> {
        let mut first = BTreeMap::<ProducerId, (CaptureId, Condition)>::new();
        for (&dependency, &when) in &self.dependencies {
            if let ProducerId::BlockOutput { block, output } = dependency.producer
                && flow.blocks[block].branch_count() > 0
                // A for cycle's item is an ordinary body-local wire.
                && !flow.takes_next_item(block)
                && !merges
                    .iter()
                    .any(|merge| merge.wire == flow.blocks[block].outputs[output])
            {
                first
                    .entry(dependency.producer)
                    .or_insert((dependency.capture, when));
            }
        }
        let missing = first
            .into_iter()
            .filter_map(|(producer, (capture, when))| {
                let absent = self.conditions.minus(self.produced(producer), when);
                self.has(absent).then_some(capture)
            })
            .min();
        if let Some(capture) = missing {
            return Err(Error::new(
                flow.blocks[capture.block].inputs[capture.input]
                    .ident
                    .span(),
                "a kaalang branch output must reach its first consumer whenever that output is selected",
            ));
        }
        Ok(())
    }
}
