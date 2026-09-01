//! Resolves parsed wires and builds the flow model.

use std::collections::HashSet;

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Flow};

mod action;
mod choice;
mod merge;
mod question;

/// Resolves a parsed flow's wires and settles terminality.
pub(crate) fn flow(flow: &mut Flow) -> Result<()> {
    validate_wires(&flow.sources, &flow.blocks)?;
    mark_terminals(&mut flow.blocks)
}

/// Checks that every input names an earlier producer and settles local name conflicts.
fn validate_wires(sources: &[Ident], blocks: &[Block]) -> Result<()> {
    let mut source_names = HashSet::new();
    for source in sources {
        if !source_names.insert(source.clone()) {
            return Err(Error::new(source.span(), "duplicate Contour source wire"));
        }
    }
    let mut producers = source_names.clone();

    for block in blocks {
        let mut seen_inputs = HashSet::new();
        for input in &block.inputs {
            if !seen_inputs.insert(input.ident.clone()) {
                return Err(Error::new(
                    input.ident.span(),
                    "duplicate Contour block input",
                ));
            }
            if !producers.contains(&input.ident) {
                return Err(Error::new(
                    input.ident.span(),
                    "a Contour block input must name a source wire or an earlier block output",
                ));
            }
        }

        let mut seen_outputs = HashSet::new();
        for output in &block.outputs {
            if !seen_outputs.insert(output.clone()) {
                return Err(Error::new(output.span(), "duplicate Contour block output"));
            }
            if source_names.contains(output) {
                return Err(Error::new(
                    output.span(),
                    "a Contour block output must not reuse a source wire name",
                ));
            }
            producers.insert(output.clone());
        }
    }

    Ok(())
}

/// Requires every output to have a consumer, except the single output that
/// makes an action terminal.
fn mark_terminals(blocks: &mut [Block]) -> Result<()> {
    let consumed = consumed_wires(blocks);

    for block in blocks {
        let unconsumed = block
            .outputs
            .iter()
            .filter(|output| !consumed.contains(*output))
            .collect::<Vec<_>>();

        let terminal = match block.kind {
            BlockKind::Action => action::terminal(&block.outputs, &unconsumed),
            BlockKind::Question => question::terminal(&unconsumed),
            BlockKind::Choice => choice::terminal(&unconsumed),
            BlockKind::Merge => merge::terminal(&unconsumed),
        }?;
        block.terminal = terminal;
    }

    Ok(())
}

/// Collects every wire name that some block reads.
fn consumed_wires(blocks: &[Block]) -> HashSet<Ident> {
    blocks
        .iter()
        .flat_map(|block| &block.inputs)
        .map(|input| input.ident.clone())
        .collect()
}
