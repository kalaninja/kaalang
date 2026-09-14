//! Reports a route that reaches the implicit completion boundary without return.

use syn::Error;

use crate::model::Flow;

pub(super) fn missing_return(flow: &Flow) -> Error {
    Error::new(
        flow.end_span(),
        "this kaalang execution reaches the end of the flow without `return`",
    )
}
