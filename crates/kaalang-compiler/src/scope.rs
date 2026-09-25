//! Gives cycle-local wires distinct internal keys, keeping authored spellings
//! in captures and output patterns, and records the outer wires each cycle's
//! body captures.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{BlockKind, Flow, Input};

#[derive(Default)]
struct Scope {
    /// The wires visible where the cycle is declared, or the flow inputs.
    inherited: BTreeMap<Ident, Ident>,
    local: BTreeMap<Ident, Ident>,
}

pub(crate) fn resolve(flow: &mut Flow) -> Result<()> {
    let mut used = flow
        .flow_inputs
        .iter()
        .cloned()
        .chain(
            flow.blocks
                .iter()
                .flat_map(|block| block.outputs.iter().cloned()),
        )
        .chain(
            flow.blocks
                .iter()
                .flat_map(|block| block.inputs.iter().map(|input| input.ident.clone())),
        )
        .collect::<BTreeSet<_>>();
    let mut scopes = BTreeMap::from([(
        None,
        Scope {
            inherited: flow
                .flow_inputs
                .iter()
                .map(|name| (name.clone(), name.clone()))
                .collect(),
            local: BTreeMap::new(),
        },
    )]);
    let mut serial = 0;
    for (index, block) in flow.blocks.iter_mut().enumerate() {
        let scope = scopes
            .get_mut(&block.parent)
            .expect("the enclosing scope was visited");
        for input in &mut block.inputs {
            if let Some(key) = scope
                .local
                .get(&input.ident)
                .or_else(|| scope.inherited.get(&input.ident))
            {
                input.ident = key.clone();
                input.ident.set_span(input.alias.span());
            }
        }
        // A cycle's own outputs exist only once it completes, so its body
        // inherits what was visible before them.
        let body = (block.kind == BlockKind::Loop).then(|| Scope {
            inherited: scope
                .inherited
                .iter()
                .chain(&scope.local)
                .map(|(name, key)| (name.clone(), key.clone()))
                .collect(),
            local: BTreeMap::new(),
        });
        for output in &mut block.outputs {
            if block.parent.is_some() && scope.inherited.contains_key(output) {
                return Err(Error::new(
                    output.span(),
                    "a cycle-local output must not shadow a wire visible outside the cycle",
                ));
            }
            let name = output.clone();
            let key = scope.local.entry(name.clone()).or_insert_with(|| {
                if block.parent.is_none() {
                    return name.clone();
                }
                fresh(&name, &mut used, &mut serial)
            });
            *output = key.clone();
            output.set_span(name.span());
        }
        if let Some(body) = body {
            scopes.insert(Some(index), body);
        }
    }
    derive_cycle_inputs(flow);
    Ok(())
}

/// Appends to each cycle the outer wires its body captures, nested cycles
/// included, after its gate and in source order (RFC 0006 §7.5).
fn derive_cycle_inputs(flow: &mut Flow) {
    let derived = flow
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(header, block)| {
            let end = block.loop_end?;
            let body = &flow.blocks[header + 1..end];
            let local = body
                .iter()
                .flat_map(|block| &block.outputs)
                .collect::<BTreeSet<_>>();
            let mut seen = block
                .inputs
                .iter()
                .map(|gate| &gate.ident)
                .collect::<BTreeSet<_>>();
            let inputs = body
                .iter()
                .flat_map(|block| &block.inputs)
                .filter(|input| !local.contains(&input.ident) && seen.insert(&input.ident))
                .map(|input| Input {
                    borrowed: false,
                    mutable: false,
                    ident: input.ident.clone(),
                    alias: input.alias.clone(),
                    derived: true,
                })
                .collect::<Vec<_>>();
            Some((header, inputs))
        })
        .collect::<Vec<_>>();
    for (header, inputs) in derived {
        flow.blocks[header].inputs.extend(inputs);
    }
}

fn fresh(name: &Ident, used: &mut BTreeSet<Ident>, serial: &mut usize) -> Ident {
    loop {
        let key = Ident::new(&format!("__kaalang_scoped_{serial}"), name.span());
        *serial += 1;
        if used.insert(key.clone()) {
            return key;
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn a_cycle_lists_its_gate_then_each_outer_capture_once() {
        let mut flow = crate::parse::flow(&parse_quote! {
            fn probe(a: u32, b: u32, go: bool) {
                #[cycle("Run the outer cycle.")]
                |go| {
                    #[action("Use b and the gate.")]
                    |b, go| {};

                    #[action("Produce a local.")]
                    let local = || 1;

                    #[cycle("Run the inner cycle.")]
                    {
                        #[action("Use a, b and the local.")]
                        |&a, &mut b, local| {};

                        break;
                    };

                    break;
                };

                return;
            }
        })
        .expect("the flow parses");
        super::resolve(&mut flow).expect("the flow resolves");
        let inputs = |header: usize| {
            flow.blocks[header]
                .inputs
                .iter()
                .map(|input| (input.alias.to_string(), input.derived))
                .collect::<Vec<_>>()
        };
        let named = |names: &[(&str, bool)]| {
            names
                .iter()
                .map(|&(name, derived)| (name.to_owned(), derived))
                .collect::<Vec<_>>()
        };
        assert_eq!(inputs(0), named(&[("go", false), ("b", true), ("a", true)]));
        assert_eq!(
            inputs(3),
            named(&[("a", true), ("b", true), ("local", true)])
        );
    }
}
