//! Captures exit inputs immediately before leaving the target native loop.

use proc_macro2::TokenStream;
use quote::quote_spanned;

use super::{Bindings, input_bindings, loop_label, transfer_value};
use kaalang_model::Flow;

pub(super) fn emit(flow: &Flow, bindings: &Bindings, index: usize, target: usize) -> TokenStream {
    let block = &flow.blocks[index];
    let captures = input_bindings(&block.inputs, bindings);
    let value = transfer_value(&block.body);
    let label = loop_label(target, block.span);
    quote_spanned! {block.span=>
        #[allow(unused_mut)]
        {
            #captures
            break #label #value;
        }
    }
}
