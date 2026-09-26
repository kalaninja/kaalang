//! Lowers locally verified stage plans into a bounded Rust dispatcher.

use std::collections::HashSet;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{ItemFn, Lifetime, Result};

use super::{Bindings, block_body, input_bindings, parameters};
use crate::{Analysis, ExecutionPlan, Flow, FlowKind};

fn state_name() -> Ident {
    Ident::new("__kaalang_state", Span::mixed_site())
}

/// A balanced sum uses existing enum variants without introducing type names
/// into authored scopes. The same path constructs and matches a stage payload.
fn variant(index: usize, count: usize, value: TokenStream2) -> TokenStream2 {
    if count == 1 {
        return value;
    }
    let left = count.div_ceil(2);
    if index < left {
        let value = variant(index, left, value);
        quote!(::core::result::Result::Ok(#value))
    } else {
        let value = variant(index - left, count - left, value);
        quote!(::core::result::Result::Err(#value))
    }
}

fn copy_value(value: &TokenStream2, span: Span) -> TokenStream2 {
    let input = Ident::new("__kaalang_entry_value", Span::mixed_site().located_at(span));
    let check = Ident::new("__kaalang_copy_entry", Span::mixed_site());
    quote_spanned!(span=> {
        let #input = #value;
        {
            const fn #check<T: ::core::marker::Copy>(value: T) -> T { value }
            #check(#input)
        }
    })
}

fn dispatch_label() -> Lifetime {
    Lifetime::new("'__kaalang_dispatch", Span::mixed_site())
}

fn prepare_label() -> Lifetime {
    Lifetime::new("'__kaalang_prepare", Span::mixed_site())
}

pub(super) fn transition(flow: &Flow, bindings: &Bindings, index: usize) -> TokenStream2 {
    let block = &flow.blocks[index];
    let target = block.transition_target.expect("a stage transition");
    let captures = input_bindings(&block.inputs, bindings);
    let value = block_body(&block.body);
    let value = if bindings.const_stages {
        copy_value(&value, block.span)
    } else {
        value
    };
    let destination = variant(target, bindings.stages, value);
    let state = state_name();
    match &flow.kind {
        FlowKind::Preparation => {
            let label = prepare_label();
            quote_spanned!(block.span=> {
                #captures
                let #state = #destination;
                break #label #state;
            })
        }
        FlowKind::Stage { .. } => {
            let label = dispatch_label();
            quote_spanned!(block.span=> {
                #captures
                #state = #destination;
                continue #label;
            })
        }
        FlowKind::Plain => unreachable!("plain flows have no stage transitions"),
    }
}

fn bindings(analysis: &Analysis, preparation: &Bindings, index: usize) -> Bindings {
    let mut bindings = Bindings::new(analysis);
    bindings.stages = preparation.stages;
    bindings.const_stages = preparation.const_stages;
    let FlowKind::Stage { entry, .. } = &analysis.flow.kind else {
        unreachable!("a stage analysis carries its entry")
    };
    for (wire, value) in &mut bindings.wires {
        if wire != entry
            && analysis.flow.flow_inputs.contains(wire)
            && let Some(outer) = preparation.wires.get(wire)
        {
            *value = outer.clone();
        } else if wire != "self" {
            *value = Ident::new(
                &format!("__kaalang_stage_{index}_{value}"),
                Span::mixed_site().located_at(wire.span()),
            );
        }
    }
    bindings
}

/// Keep preparation's outer owners alive, including those only borrowed by
/// another prepared wire. Branch and iteration locals retain their own scopes.
fn outer_wires(flow: &Flow, mut plan: &ExecutionPlan, wires: &mut HashSet<Ident>) {
    loop {
        match plan {
            ExecutionPlan::End { body, .. } => plan = body,
            ExecutionPlan::Action { index, next } | ExecutionPlan::Call { index, next } => {
                wires.extend(flow.blocks[*index].outputs.iter().cloned());
                plan = next;
            }
            ExecutionPlan::Loop {
                index, branches, ..
            } if flow.blocks[*index].branch_count() == 0 => {
                wires.extend(flow.blocks[*index].outputs.iter().cloned());
                let Some(branch) = branches.first() else {
                    return;
                };
                plan = &branch.plan;
            }
            ExecutionPlan::Question { joins, .. }
            | ExecutionPlan::Choice { joins, .. }
            | ExecutionPlan::Loop { joins, .. } => {
                let Some(join) = joins.last() else { return };
                wires.extend(join.wires.iter().cloned());
                plan = &join.next;
            }
            ExecutionPlan::Return { .. }
            | ExecutionPlan::Yield { .. }
            | ExecutionPlan::Continue { .. }
            | ExecutionPlan::Export { .. } => return,
        }
    }
}

pub(super) fn expand(mut function: ItemFn, analysis: &Analysis) -> Result<ItemFn> {
    let preparation_topology = crate::project(analysis, false);
    crate::construct(analysis, &preparation_topology)?;
    for stage in &analysis.stages {
        let topology = crate::project(&stage.analysis, false);
        crate::construct(&stage.analysis, &topology)?;
    }

    let mut prepared = Bindings::new(analysis);
    prepared.const_stages = function.sig.constness.is_some();
    outer_wires(
        &analysis.flow,
        &analysis.execution_plan,
        &mut prepared.hoisted,
    );
    for stage in &analysis.stages {
        prepared.mutable.extend(
            stage
                .analysis
                .flow
                .blocks
                .iter()
                .flat_map(|block| &block.inputs)
                .filter(|input| input.borrowed && input.mutable)
                .map(|input| input.ident.clone())
                .filter(|wire| analysis.common_wires.contains(wire)),
        );
    }
    let prologue = parameters::emit(&function, &prepared);
    // Declaration order determines Rust's drop order, even when initialization
    // happens later inside the preparation block.
    let mut declared = HashSet::new();
    let declarations = analysis
        .flow
        .blocks
        .iter()
        .flat_map(|block| &block.outputs)
        .filter(|wire| prepared.hoisted.contains(*wire) && declared.insert(*wire))
        .map(|wire| {
            let binding = prepared.wire(wire);
            let mutable = prepared.mutability(wire);
            quote!(let #mutable #binding;)
        });
    let preparation = super::flow(&analysis.flow, &analysis.execution_plan, &prepared);
    let state = state_name();
    let prep_label = prepare_label();
    let dispatch_label = dispatch_label();
    let arms = analysis.stages.iter().enumerate().map(|(index, stage)| {
        let entry = &stage.entry;
        let bindings = bindings(&stage.analysis, &prepared, index);
        let value = bindings.wire(entry);
        let pattern = variant(index, prepared.stages, quote!(#value));
        let body = super::flow(
            &stage.analysis.flow,
            &stage.analysis.execution_plan,
            &bindings,
        );
        quote!(#pattern => { #body })
    });
    *function.block = syn::parse2(quote!({
        #[allow(clippy::used_underscore_binding, unused_mut)]
        {
            #prologue
            #(#declarations)*
            let mut #state = #prep_label: { #preparation };
            #dispatch_label: loop {
                match #state { #(#arms,)* }
            }
        }
    }))?;
    Ok(function)
}
