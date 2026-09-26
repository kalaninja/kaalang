//! Reports a route that reaches the implicit completion boundary without return.

use syn::Error;

use crate::model::{Flow, FlowKind};

pub(super) fn missing_return(flow: &Flow) -> Error {
    let message = match flow.kind {
        FlowKind::Plain => "this kaalang execution reaches the end of the flow without `return`",
        FlowKind::Preparation => {
            "this kaalang preparation route reaches the stage section without selecting a stage signal"
        }
        FlowKind::Stage { .. } => {
            "this kaalang stage route reaches the end of its body without a transition or `return`"
        }
    };
    Error::new(flow.end_span(), message)
}
