//! Resolves parsed wires against their producers.

use std::collections::{HashMap, HashSet};

use syn::{Error, Result};

use crate::model::Flow;

/// Resolves a parsed flow's wires: every reachable input names an earlier
/// producer and declarations do not collide with themselves.
pub(crate) fn flow(flow: &Flow) -> Result<()> {
    let mut flow_inputs = HashSet::new();
    for input in &flow.flow_inputs {
        if !flow_inputs.insert(input.clone()) {
            return Err(Error::new(input.span(), "duplicate kaalang flow input"));
        }
    }
    let mut producers = flow_inputs.clone();
    let mut earlier_inputs = HashSet::new();
    let mut output_mutability = HashMap::new();

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
            if input.borrowed
                && input.mutable
                && output_mutability.get(&input.ident) == Some(&false)
            {
                return Err(Error::new(
                    input.alias.span(),
                    "a mutable capture requires the output wire to be declared with `mut`",
                ));
            }
        }

        let mut seen_outputs = HashSet::new();
        for (index, output) in block.outputs.iter().enumerate() {
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
            let mutable = block.output_binding(index).mutability.is_some();
            if let Some(previous) = output_mutability.insert(output.clone(), mutable)
                && previous != mutable
            {
                return Err(Error::new(
                    output.span(),
                    "alternative producers of a kaalang wire must declare the same mutability",
                ));
            }
            producers.insert(output.clone());
        }
        for input in &block.inputs {
            if let Some(binding) = &input.binding {
                producers.insert(binding.clone());
                output_mutability.insert(binding.clone(), input.mutable);
            }
        }
        earlier_inputs.extend(block.inputs.iter().map(|input| input.ident.clone()));
    }

    Ok(())
}
