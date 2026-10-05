//! Keeps every block on a path it belongs to. After a selection, the blocks a
//! source order runs must stay inside the selected branch, or inside a nested
//! continuation of it, until a wire merge joins that branch with the others.

use std::{
    cell::OnceCell,
    collections::{BTreeMap, BTreeSet},
};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, BranchSelection, Execution, Flow, ProducerId, WireMerge};

/// The selections one producer occurrence sits inside: what its block inherited
/// by capture, plus the branch it selects when the block is a question or choice.
fn occurrence(
    flow: &Flow,
    inherited: &BTreeSet<BranchSelection>,
    block: usize,
    output: usize,
) -> BTreeSet<BranchSelection> {
    let mut selections = inherited.clone();
    if flow.blocks[block].branch_count() > 0 {
        selections.insert(BranchSelection {
            block,
            branch: output,
        });
    }
    selections
}

/// The case of a for cycle's hidden choice that every block of its body runs
/// in, apart from the choice itself and the export it skips to.
pub(crate) fn item_case(flow: &Flow, block: usize) -> Option<BranchSelection> {
    flow.next_item(block).map(|next| BranchSelection {
        block: next,
        branch: 0,
    })
}

/// The selections each block inherits by capture. A block belongs to a branch
/// when it captures the branch output itself or a wire produced inside that
/// branch; a wire with alternative producers carries only what all of them
/// agree on, which is how a merge hands the branch back to the common path.
pub(crate) fn ancestry(flow: &Flow) -> Vec<BTreeSet<BranchSelection>> {
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
        }
        inherited.extend(item_case(flow, index));
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

/// The selections each block carries in one execution: those of the producers
/// it actually captured, not only those every alternative shares. A partial
/// merge therefore keeps its branch on the value until a later merge closes it.
fn carried_in(flow: &Flow, execution: &Execution) -> Vec<BTreeSet<BranchSelection>> {
    let mut captured = vec![Vec::new(); flow.blocks.len()];
    for dependency in &execution.dependencies {
        if let ProducerId::BlockOutput { block, output } = dependency.producer {
            captured[dependency.capture.block].push((block, output));
        }
    }
    let mut carried = vec![BTreeSet::new(); flow.blocks.len()];
    for &block in &execution.blocks {
        let mut selections = flow.blocks[block]
            .parent
            .map(|parent| carried[parent].clone())
            .unwrap_or_default();
        selections.extend(item_case(flow, block));
        for &(producer, output) in &captured[block] {
            selections.extend(occurrence(flow, &carried[producer], producer, output));
        }
        carried[block] = selections;
    }
    carried
}

/// One implicit junction seen from source order: its last alternative, the
/// work completing its branches, and each producer occurrence's selections.
struct Junction<'a> {
    flow: &'a Flow,
    merge: &'a WireMerge,
    position: usize,
    owners: &'a [(usize, Vec<usize>)],
    producers: Vec<(ProducerId, BTreeSet<BranchSelection>)>,
    /// Whether this junction covers every execution shared by a block and a
    /// selection, by `(block, selection.block)`. The answer reads the whole
    /// execution list, and neither the asking execution nor the selected
    /// branch takes part in it, so one pass over a flow answers each pair once.
    shared: BTreeMap<(usize, usize), bool>,
}

impl<'a> Junction<'a> {
    fn of(
        flow: &'a Flow,
        ancestry: &[BTreeSet<BranchSelection>],
        merge: &'a WireMerge,
        owners: &'a [(usize, Vec<usize>)],
    ) -> Self {
        let mut position = 0;
        let mut producers = Vec::with_capacity(merge.producers.len());
        for &producer in &merge.producers {
            let ProducerId::BlockOutput { block, output } = producer else {
                unreachable!("a wire merge combines block outputs")
            };
            // A cycle's output is ready only once its whole body has run.
            let ready = flow.blocks[block].cycle_end.map_or(block, |end| end - 1);
            position = position.max(ready);
            producers.push((producer, occurrence(flow, &ancestry[block], block, output)));
        }
        Self {
            flow,
            merge,
            position,
            owners,
            producers,
            shared: BTreeMap::new(),
        }
    }

    /// A completed merge admits a block only within the branches it joins.
    /// The producer that ran carries its selections through earlier partial
    /// merges.
    fn closes(
        &mut self,
        execution: &Execution,
        carried: &OnceCell<Vec<BTreeSet<BranchSelection>>>,
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
        let Some((producer, occurrence)) = self
            .producers
            .iter()
            .find(|&&(producer, _)| self.flow.produces(execution, producer))
        else {
            return false;
        };
        let ProducerId::BlockOutput { block: ran, .. } = *producer else {
            unreachable!("a wire merge combines block outputs")
        };
        // The occurrence keeps only what every alternative shares, a subset of
        // what the value that ran carries: selections made before it ran.
        let reached = || {
            occurrence.contains(&selection)
                || (selection.block < ran
                    && carried.get_or_init(|| carried_in(self.flow, execution))[ran]
                        .contains(&selection))
        };
        let closes = self
            .owners
            .iter()
            .any(|&(owner, _)| owner == selection.block)
            || (reached()
                && self
                    .producers
                    .iter()
                    .any(|(_, other)| !other.contains(&selection)));
        if !closes {
            return false;
        }
        // A partial merge admits only its own continuation. A block
        // shared with another still-separate group needs a larger merge.
        if let Some(&shared) = self.shared.get(&(block, selection.block)) {
            return shared;
        }
        let shared = executions
            .iter()
            .filter(|other| other.participates(block) && other.participates(selection.block))
            .all(|other| {
                self.producers
                    .iter()
                    .any(|&(producer, _)| self.flow.produces(other, producer))
            });
        self.shared.insert((block, selection.block), shared);
        shared
    }
}

/// Every participating block stays inside each branch selected above it until a
/// junction below that selection and above the block joins it with the others.
pub(super) fn flow(
    flow: &Flow,
    executions: &[Execution],
    merges: &[WireMerge],
    owners: &[Vec<(usize, Vec<usize>)>],
    ancestry: &[BTreeSet<BranchSelection>],
    frames: &super::frame::Frames<'_>,
) -> Result<()> {
    let mut junctions = merges
        .iter()
        .zip(owners)
        .map(|(merge, owners)| Junction::of(flow, ancestry, merge, owners))
        .collect::<Vec<_>>();
    let end = flow.blocks.len() - 1;
    // The end of a for body has no capture to name its branch: it belongs to
    // whichever branch arrives, so a selection stays separate there only when
    // more than one of its branches reaches it. Routes that diverge never do.
    let mut arriving = BTreeMap::<(usize, usize), BTreeSet<usize>>::new();
    for execution in executions {
        for &block in execution
            .blocks
            .iter()
            .filter(|&&block| flow.ends_iteration(block))
        {
            for selection in &execution.branches {
                arriving
                    .entry((block, selection.block))
                    .or_default()
                    .insert(selection.branch);
            }
        }
    }
    let mut offending = None::<(usize, usize)>;
    for execution in executions {
        // Built on the first junction that needs it; most pairs never do.
        let carried = OnceCell::new();
        for &block in execution.blocks.iter().filter(|&&block| block < end) {
            let frame = frames.frame(block);
            for selection in execution
                .branches
                .iter()
                .filter(|s| s.block < block && frames.visible(frame, s.block))
            {
                if super::cycle::closed_before(flow, selection.block, block)
                    || ancestry[block].contains(selection)
                    || arriving
                        .get(&(block, selection.block))
                        .is_some_and(|branches| branches.len() < 2)
                    || junctions.iter_mut().any(|junction| {
                        junction.closes(execution, &carried, block, *selection, executions)
                    })
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
    Err(violation(flow, block, selection))
}

pub(crate) fn violation(flow: &Flow, block: usize, selection: usize) -> Error {
    let declaration = &flow.blocks[selection];
    let kind = crate::parse::noun(declaration.kind);
    let described = declaration
        .description
        .as_deref()
        .expect("every selection is authored with a description");
    // A boundary consumer is not authored: it exports at the end of the body.
    let message = if flow.blocks[block].transition_target.is_some() {
        let signal = flow.wire_name(&flow.blocks[block].inputs[0].ident);
        format!(
            "this kaalang stage transition exports `{signal}` while the branches of the {kind} `{described}` are still separate; merge those branches before the transition"
        )
    } else if flow.ends_iteration(block) {
        format!(
            "this kaalang for cycle ends an iteration while the branches of the {kind} `{described}` are still separate; merge those branches before the end of its body"
        )
    } else if flow.blocks[block].kind == BlockKind::Export {
        format!(
            "this kaalang cycle exports `{}` while the branches of the {kind} `{described}` are still separate; merge those branches before the end of its body",
            super::exported_name(flow, block)
        )
    } else {
        format!(
            "this kaalang block runs while the branches of the {kind} `{described}` are still separate; give it an input from one branch, or merge those branches above it"
        )
    };
    Error::new(flow.blocks[block].span, message)
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
}
