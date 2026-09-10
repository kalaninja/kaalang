//! Gives loop-local wires distinct internal keys, keeping authored spellings
//! in capture aliases and output patterns for code generation and diagrams.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;
use syn::{Error, Result};

use crate::model::{Flow, RESULT_WIRE};

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
            }
        }
        for output in &mut block.outputs {
            if block.parent.is_some()
                && *output != RESULT_WIRE
                && scope.inherited.contains_key(output)
            {
                return Err(Error::new(
                    output.span(),
                    "a loop-local output must not redeclare an enclosing wire",
                ));
            }
            let name = output.clone();
            let key = scope.local.entry(name.clone()).or_insert_with(|| {
                if block.parent.is_none() || name == RESULT_WIRE {
                    return name.clone();
                }
                loop {
                    let key = Ident::new(&format!("__kaalang_scoped_{serial}"), name.span());
                    serial += 1;
                    if used.insert(key.clone()) {
                        return key;
                    }
                }
            });
            *output = key.clone();
            output.set_span(name.span());
        }
        if block.loop_end.is_some() {
            let inherited = scope
                .inherited
                .iter()
                .chain(&scope.local)
                .map(|(name, key)| (name.clone(), key.clone()))
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
