//! Emits a native unconditional loop.

use proc_macro2::TokenStream;
use quote::quote_spanned;

use super::{Bindings, loop_label};
use kaalang_model::{ExecutionPlan, Flow};

pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    index: usize,
    body: &ExecutionPlan,
) -> TokenStream {
    let block = &flow.blocks[index];
    let body = super::flow(flow, body, bindings);
    let label = loop_label(index, block.span);
    quote_spanned! {block.span=>
        #[allow(unused_labels, clippy::never_loop)]
        #label: loop { #body }
    }
}
