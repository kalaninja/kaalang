//! Validates convergence from dependency chains, independently of lowering.

use std::collections::{BTreeMap, BTreeSet};

use syn::{Error, Result};

use crate::model::{BlockKind, Execution, Flow, ProducerId};

pub(super) fn flow(flow: &Flow, executions: &[Execution]) -> Result<()> {
    let precedence = executions
        .iter()
        .map(|execution| predecessors(flow.blocks.len(), execution))
        .collect::<Vec<_>>();
    let mut unavailable = None;
    for (brancher, declaration) in flow.blocks.iter().enumerate() {
        if !matches!(declaration.kind, BlockKind::Question | BlockKind::Choice) {
            continue;
        }
        let mut branch_sets = vec![BTreeSet::new(); flow.blocks.len()];
        for (execution, preceding) in executions.iter().zip(&precedence) {
            let Some(selection) = execution.branches.iter().find(|s| s.block == brancher) else {
                continue;
            };
            for &block in &execution.blocks {
                if preceding[block].contains(&brancher) {
                    branch_sets[block].insert(selection.branch);
                }
            }
        }
        let mut groups = BTreeMap::<Vec<usize>, BTreeSet<usize>>::new();
        for (block, branches) in branch_sets.iter().enumerate() {
            if branches.len() >= 2 {
                groups
                    .entry(branches.iter().copied().collect())
                    .or_default()
                    .insert(block);
            }
        }
        let groups = groups.into_iter().collect::<Vec<_>>();
        if declaration.kind == BlockKind::Choice {
            super::choice::validate_groups(&declaration.outputs, &groups)?;
        }
        for (branches, shared) in &groups {
            for (execution, preceding) in executions.iter().zip(&precedence) {
                for &block in &execution.blocks {
                    if !preceding[block].is_disjoint(shared)
                        && branches
                            .iter()
                            .any(|branch| !branch_sets[block].contains(branch))
                    {
                        unavailable =
                            Some(unavailable.map_or(block, |earliest: usize| earliest.min(block)));
                    }
                }
            }
        }
    }
    if let Some(block) = unavailable {
        return Err(Error::new(
            flow.blocks[block].span,
            "a kaalang block after a convergence point must belong to every branch that converges there",
        ));
    }
    Ok(())
}

/// Authored order is topological: every producer precedes all its consumers.
// ponytail: cloning ancestor sets costs O(blocks²) per execution and the
// caller runs it for every execution and brancher; switch to bitsets computed
// once per execution if flows reach hundreds of blocks with many executions.
fn predecessors(blocks: usize, execution: &Execution) -> Vec<BTreeSet<usize>> {
    let mut preceding = vec![BTreeSet::new(); blocks];
    for &block in &execution.blocks {
        for dependency in execution
            .dependencies
            .iter()
            .filter(|d| d.capture.block == block)
        {
            if let ProducerId::BlockOutput {
                block: producer, ..
            } = dependency.producer
            {
                let ancestors = preceding[producer].clone();
                preceding[block].extend(ancestors);
                preceding[block].insert(producer);
            }
        }
    }
    preceding
}
