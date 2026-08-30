//! Resolves parsed wires and builds the flow model.

use std::collections::HashSet;

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Block, BlockKind, Flow, ParsedFlow};

/// Resolves a parsed flow and settles terminality.
pub(crate) fn flow(parsed: ParsedFlow) -> Result<Flow> {
    let ParsedFlow {
        sources,
        mut blocks,
    } = parsed;
    validate_wires(&sources, &blocks)?;
    mark_terminals(&mut blocks)?;

    Ok(Flow::new(sources, blocks))
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
        let mut seen_inputs = HashSet::new();
        for input in &block.inputs {
            let name = input.ident.to_string();
            if !seen_inputs.insert(name.clone()) {
                return Err(Error::new(
                    input.ident.span(),
                    "duplicate Contour block input",
                ));
            }
            if !producers.contains(&name) {
                return Err(Error::new(
                    input.ident.span(),
                    "a Contour block input must name a source wire or an earlier block output",
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
                    "every Contour question output must have a consumer",
                ));
            }
            BlockKind::Choice if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "every Contour choice output must have a consumer",
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
