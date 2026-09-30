//! Lowers locally verified stage plans into a bounded Rust dispatcher.

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
fn variant(index: usize, count: usize, value: TokenStream2, span: Span) -> TokenStream2 {
    if count == 1 {
        return value;
    }
    let left = count.div_ceil(2);
    if index < left {
        let value = variant(index, left, value, span);
        quote_spanned!(span=> ::core::result::Result::Ok(#value))
    } else {
        let value = variant(index - left, count - left, value, span);
        quote_spanned!(span=> ::core::result::Result::Err(#value))
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
    let destination = variant(target, bindings.stages, value, block.span);
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

/// Only the final selection leaves a preparation scope. Earlier initializers
/// share the dispatcher's scope, preserving owners and temporary lifetimes.
pub(super) fn prepare(
    bindings: &Bindings,
    plan: &ExecutionPlan,
    body: TokenStream2,
) -> TokenStream2 {
    let index = match plan {
        ExecutionPlan::Question { index, .. }
        | ExecutionPlan::Choice { index, .. }
        | ExecutionPlan::Loop { index, .. }
        | ExecutionPlan::Return { index } => *index,
        _ => return body,
    };
    let Some((boundary, dispatcher)) = &bindings.initial_dispatch else {
        return body;
    };
    if index != *boundary {
        return body;
    }
    let state = state_name();
    let label = prepare_label();
    quote! {
        let mut #state = #label: { #body };
        #dispatcher
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
    let prologue = parameters::emit(&mut function, &prepared);
    let state = state_name();
    let dispatch_label = dispatch_label();
    let order = crate::stage::visit_order(analysis, &analysis.stages);
    let arms = order.into_iter().map(|index| {
        let stage_analysis = &analysis.stages[index];
        let entry = &stage_analysis.entry;
        let bindings = bindings(&stage_analysis.analysis, &prepared, index);
        let value = bindings.wire(entry);
        let pattern = variant(index, prepared.stages, quote!(#value), entry.span());
        let body = super::flow(
            &stage_analysis.analysis.flow,
            &stage_analysis.analysis.execution_plan,
            &bindings,
        );
        let arm_label = Lifetime::new("'__kaalang_stage_body", Span::mixed_site());
        quote!(#pattern => #arm_label: { #body })
    });
    let dispatcher = quote! {
        #dispatch_label: loop {
            match #state { #(#arms,)* }
        }
    };
    let (_, boundary) = crate::stage::preparation_scope(&analysis.flow, &analysis.execution_plan);
    prepared.initial_dispatch = Some((boundary, dispatcher));
    let preparation = super::flow(&analysis.flow, &analysis.execution_plan, &prepared);
    *function.block = syn::parse2(quote!({
        #[allow(clippy::used_underscore_binding, unused_labels, unused_mut)]
        {
            #prologue
            #preparation
        }
    }))?;
    Ok(function)
}
