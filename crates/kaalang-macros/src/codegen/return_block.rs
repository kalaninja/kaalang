//! Returns one captured value from the root flow.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;

use super::{Bindings, input_bindings, transfer_value};
use kaalang_model::Flow;

pub(super) fn emit(flow: &Flow, bindings: &Bindings, index: usize) -> TokenStream2 {
    let block = &flow.blocks[index];
    let captures = input_bindings(&block.inputs, bindings);
    let value = transfer_value(&block.body);
    quote_spanned! {block.span=>
        #[allow(unused_mut)]
        {
            #captures
            return #value;
        }
    }
}
