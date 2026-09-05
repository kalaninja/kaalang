//! Resolves parsed wires against their producers.

use std::collections::HashSet;

use syn::{Error, Result};

use crate::model::Flow;

/// Resolves a parsed flow's wires: every input names an earlier producer, and
/// declarations do not collide with themselves.
pub(crate) fn flow(flow: &Flow) -> Result<()> {
    let mut flow_inputs = HashSet::new();
    for input in &flow.flow_inputs {
        if !flow_inputs.insert(input.clone()) {
            return Err(Error::new(input.span(), "duplicate kaalang flow input"));
        }
    }
    let mut producers = flow_inputs.clone();
    let mut earlier_inputs = HashSet::new();

    for block in &flow.blocks {
        let mut seen_inputs = HashSet::new();
        for input in &block.inputs {
            if !seen_inputs.insert(input.ident.clone()) {
                return Err(Error::new(
                    input.ident.span(),
                    "duplicate kaalang block input",
                ));
            }
            if !producers.contains(&input.ident) {
                return Err(Error::new(
                    input.ident.span(),
                    "a kaalang block input must name a flow input or an earlier block output",
                ));
            }
        }

        let mut seen_outputs = HashSet::new();
        for output in &block.outputs {
            if !seen_outputs.insert(output.clone()) {
                return Err(Error::new(output.span(), "duplicate kaalang block output"));
            }
            if flow_inputs.contains(output) {
                return Err(Error::new(
                    output.span(),
                    "a kaalang block output must not reuse a flow input name",
                ));
            }
            if block.inputs.iter().any(|input| input.ident == *output) {
                return Err(Error::new(
                    output.span(),
                    "a kaalang wire must not be produced more than once in one execution",
                ));
            }
            if earlier_inputs.contains(output) {
                return Err(Error::new(
                    output.span(),
                    "every producer of a kaalang wire must be declared before its consumers",
                ));
            }
            producers.insert(output.clone());
        }
        earlier_inputs.extend(block.inputs.iter().map(|input| input.ident.clone()));
    }

    Ok(())
}
