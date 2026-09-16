//! Gives cycle interfaces and local wires distinct internal keys, keeping
//! authored spellings in captures and output patterns.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result, ext::IdentExt};

use crate::model::{BlockKind, Flow};

#[derive(Default)]
struct Scope {
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
            } else if block.parent.is_some() {
                return Err(Error::new(
                    input.ident.span(),
                    "a block inside a kaalang cycle may capture only a cycle input or an earlier local output",
                ));
            }
        }
        for output in &mut block.outputs {
            if block.parent.is_some() && scope.inherited.contains_key(output) {
                return Err(Error::new(
                    output.span(),
                    "a cycle-local output must not redeclare a cycle input",
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
        if block.kind == BlockKind::Loop {
            let inherited = block
                .inputs
                .iter_mut()
                .map(|input| {
                    let name = input.alias.unraw();
                    // The receiver is the one wire a cycle cannot rebind: Rust
                    // binds `self` only as a receiver, so it stays the same
                    // wire inside the cycle as outside it.
                    let binding = if name == "self" {
                        name.clone()
                    } else {
                        fresh(&name, &mut used, &mut serial)
                    };
                    input.binding = Some(binding.clone());
                    (name, binding)
                })
                .collect();
            scopes.insert(
                Some(index),
                Scope {
                    inherited,
                    local: BTreeMap::new(),
                },
            );
        }
    }
    Ok(())
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
