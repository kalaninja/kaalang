//! Validates parsed wires and builds the semantic graph.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, Span};
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Graph, ParsedFlow};

/// Validates wire names, settles terminality, and assigns internal bindings.
pub(crate) fn build(parsed: ParsedFlow) -> Result<Graph> {
    let ParsedFlow {
        sources,
        mut blocks,
    } = parsed;
    validate_wires(&sources, &blocks)?;
    mark_terminals(&mut blocks)?;
    let wires = internal_wires(&sources, &blocks);

    Ok(Graph::new(sources, blocks, wires))
}

/// Checks that every input names an earlier producer and that wire names are unique.
fn validate_wires(sources: &[Ident], blocks: &[Block]) -> Result<()> {
    let mut producers = HashSet::new();
    for source in sources {
        if !producers.insert(source.to_string()) {
            return Err(Error::new(source.span(), "duplicate Contour source wire"));
        }
    }

    for block in blocks {
        let mut captured = HashSet::new();
        for input in &block.inputs {
            let name = input.ident.to_string();
            if !captured.insert(name.clone()) {
                return Err(Error::new(
                    input.ident.span(),
                    "duplicate Contour block input",
                ));
            }
            if !producers.contains(&name) {
                return Err(Error::new(
                    input.ident.span(),
                    "a Contour input must reference a function parameter or an earlier block output",
                ));
            }
        }

        for output in &block.outputs {
            if !producers.insert(output.to_string()) {
                return Err(Error::new(output.span(), "duplicate Contour wire name"));
            }
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
            .filter(|output| !consumed.contains(&output.to_string()))
            .collect::<Vec<_>>();

        match block.kind {
            BlockKind::Question if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "every Contour question branch must have a consumer",
                ));
            }
            BlockKind::Choice if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "every Contour choice case must have a consumer",
                ));
            }
            BlockKind::Action if unconsumed.is_empty() => {}
            BlockKind::Action if block.outputs.len() == 1 => block.terminal = true,
            BlockKind::Action => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "a terminal Contour action must have exactly one output",
                ));
            }
            BlockKind::Merge if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "a Contour merge output must have a consumer",
                ));
            }
            BlockKind::Question | BlockKind::Choice | BlockKind::Merge => {}
        }
    }

    Ok(())
}

/// Collects every wire name that some block reads.
fn consumed_wires(blocks: &[Block]) -> HashSet<String> {
    blocks
        .iter()
        .flat_map(|block| &block.inputs)
        .map(|input| input.ident.to_string())
        .collect()
}

/// Assigns each source and output wire a hygienic Rust binding.
fn internal_wires(sources: &[Ident], blocks: &[Block]) -> HashMap<String, Ident> {
    // Source spellings are reserved for aliases inside the blocks that capture them.
    sources
        .iter()
        .chain(blocks.iter().flat_map(|block| &block.outputs))
        .enumerate()
        .map(|(index, wire)| {
            (
                wire.to_string(),
                Ident::new(
                    &format!("__contour_wire_{index}"),
                    Span::mixed_site().located_at(wire.span()),
                ),
            )
        })
        .collect()
}
