//! Keeps every block on a path it belongs to. After a selection, the blocks a
//! source order runs must stay inside the selected branch, or inside a nested
//! continuation of it, until a wire merge joins that branch with the others.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, BranchSelection, Execution, Flow, ProducerId, WireMerge};

use super::produced;

/// The selections one producer occurrence sits inside: what its block inherited
/// by capture, plus the branch it selects when the block is a question or choice.
fn occurrence(
    flow: &Flow,
    inherited: &BTreeSet<BranchSelection>,
    block: usize,
    output: usize,
) -> BTreeSet<BranchSelection> {
    let mut selections = inherited.clone();
    if branches(flow.blocks[block].kind) {
        selections.insert(BranchSelection {
            block,
            branch: output,
        });
    }
    selections
}

/// The selections each block inherits by capture. A block belongs to a branch
/// when it captures the branch output itself or a wire produced inside that
/// branch; a wire with alternative producers carries only what all of them
/// agree on, which is how a merge hands the branch back to the common path.
fn ancestry(flow: &Flow) -> Vec<BTreeSet<BranchSelection>> {
    let mut wires = BTreeMap::<Ident, BTreeSet<BranchSelection>>::new();
    for input in &flow.flow_inputs {
        wires.insert(input.clone(), BTreeSet::new());
    }
    let mut blocks: Vec<BTreeSet<BranchSelection>> = Vec::with_capacity(flow.blocks.len());
    for (index, declaration) in flow.blocks.iter().enumerate() {
        let mut inherited = declaration
            .inputs
            .iter()
            .flat_map(|input| wires.get(&input.ident).into_iter().flatten().copied())
            .collect::<BTreeSet<_>>();
        if let Some(parent) = declaration.parent {
            inherited.extend(blocks[parent].iter().copied());
            if flow.blocks[parent].kind == BlockKind::While {
                inherited.insert(BranchSelection {
                    block: parent,
                    branch: flow.blocks[parent].yes_branch(),
                });
            }
        }
        for (output, name) in declaration.outputs.iter().enumerate() {
            let occurrence = occurrence(flow, &inherited, index, output);
            // Alternative producers meet before every capture, so the merged
            // wire keeps only the selections every one of them shares.
            wires
                .entry(name.clone())
                .and_modify(|shared| shared.retain(|selection| occurrence.contains(selection)))
                .or_insert(occurrence);
        }
        blocks.push(inherited);
    }
    blocks
}

fn branches(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Question | BlockKind::Choice)
}

/// One implicit junction seen from source order: its last alternative, the
/// work completing its branches, and each producer occurrence's selections.
struct Junction<'a> {
    merge: &'a WireMerge,
    position: usize,
    owners: &'a [(usize, Vec<usize>)],
    producers: Vec<(ProducerId, BTreeSet<BranchSelection>)>,
}

impl<'a> Junction<'a> {
    fn of(
        flow: &Flow,
        ancestry: &[BTreeSet<BranchSelection>],
        merge: &'a WireMerge,
        owners: &'a [(usize, Vec<usize>)],
    ) -> Self {
        let outputs = merge.producers.iter().map(|producer| match *producer {
            ProducerId::BlockOutput { block, output } => (block, output),
            ProducerId::FlowInput(_) => unreachable!("a wire merge combines block outputs"),
        });
        let mut position = 0;
        let mut producers = Vec::with_capacity(merge.producers.len());
        for (block, output) in outputs {
            position = position.max(block);
            producers.push((
                ProducerId::BlockOutput { block, output },
                occurrence(flow, &ancestry[block], block, output),
            ));
        }
        Self {
            merge,
            position,
            owners,
            producers,
        }
    }

    /// A completed merge admits a block only within the branches it joins.
    /// Producer selection also carries ancestry through earlier partial merges.
    fn closes(
        &self,
        execution: &Execution,
        block: usize,
        selection: BranchSelection,
        executions: &[Execution],
    ) -> bool {
        // Direct consumers get the more specific merge-order diagnostic.
        if self.position >= block
            || (!self.merge.after.contains(&block)
                && self
                    .merge
                    .before
                    .iter()
                    .any(|&before| before >= block && execution.participates(before)))
        {
            return false;
        }
        let Some((_, reached)) = self
            .producers
            .iter()
            .find(|&&(producer, _)| produced(execution, producer))
        else {
            return false;
        };
        let closes = self
            .owners
            .iter()
            .any(|&(owner, _)| owner == selection.block)
            || (reached.contains(&selection)
                && self
                    .producers
                    .iter()
                    .any(|(_, other)| !other.contains(&selection)));
        // A partial merge admits only its own continuation. A block
        // shared with another still-separate group needs a larger merge.
        closes
            && executions
                .iter()
                .filter(|other| other.participates(block) && other.participates(selection.block))
                .all(|other| {
                    self.producers
                        .iter()
                        .any(|&(producer, _)| produced(other, producer))
                })
    }
}

/// Every participating block stays inside each branch selected above it until a
/// junction below that selection and above the block joins it with the others.
pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    merges: &[WireMerge],
    owners: &[Vec<(usize, Vec<usize>)>],
) -> Result<()> {
    let ancestry = ancestry(flow);
    let junctions = merges
        .iter()
        .zip(owners)
        .map(|(merge, owners)| Junction::of(flow, &ancestry, merge, owners))
        .collect::<Vec<_>>();
    let end = flow.blocks.len() - 1;
    let mut offending = None::<(usize, usize)>;
    for execution in executions {
        for &block in execution.blocks.iter().filter(|&&block| block < end) {
            for selection in execution.branches.iter().filter(|s| s.block < block) {
                if super::while_loop::closed_before(flow, selection.block, block)
                    || ancestry[block].contains(selection)
                    || junctions
                        .iter()
                        .any(|junction| junction.closes(execution, block, *selection, executions))
                {
                    continue;
                }
                let violation = (block, selection.block);
                offending = Some(offending.map_or(violation, |old| violation.min(old)));
            }
        }
    }
    let Some((block, selection)) = offending else {
        return Ok(());
    };
    let declaration = &flow.blocks[selection];
    let kind = match declaration.kind {
        BlockKind::Choice => "choice",
        _ => "question",
    };
    let described = declaration
        .description
        .as_deref()
        .expect("every selection is authored with a description");
    Err(Error::new(
        flow.blocks[block].span,
        format!(
            "this kaalang block runs while the branches of the {kind} `{described}` are still separate; give it an input from one branch, or merge those branches above it"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use crate::tests::{branch_placement, fixture, message};

    #[test]
    fn common_work_waits_for_branch_completion() {
        let source = include_str!(
            "../../../kaalang/tests/wire/compile_fail/common_work_before_branch_completion.rs"
        );
        let mut function = fixture(source, "invalid");
        assert_eq!(
            message(&function),
            branch_placement("question", "Choose the value.")
        );
        function.block.stmts.swap(3, 4);
        crate::build(&function).expect("common work may follow the completed merge");
    }

    #[test]
    fn common_work_cannot_join_disjoint_partial_groups() {
        let source = include_str!(
            "../../../kaalang/tests/wire/compile_fail/common_work_after_disjoint_merges.rs"
        );
        assert_eq!(
            message(&fixture(source, "invalid")),
            branch_placement("choice", "Which source?")
        );
    }
}
