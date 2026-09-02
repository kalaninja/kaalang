//! Validates the skewer topology of a choice's cases.

use proc_macro2::Ident;
use syn::{Error, Result};

/// Rejects a case that reaches End between two cases that enter a shared continuation,
/// because no skewer order can draw that crossing topology.
pub(super) fn adjacent_branches(continuing: &[bool], outputs: &[Ident]) -> Result<()> {
    let Some(first) = continuing.iter().position(|branch| *branch) else {
        return Ok(());
    };
    let last = continuing
        .iter()
        .rposition(|branch| *branch)
        .expect("a continuing branch was just found");
    let Some(offset) = continuing[first..last].iter().position(|branch| !*branch) else {
        return Ok(());
    };

    Err(Error::new(
        outputs[first + offset].span(),
        "a kaalang case that reaches End must not separate cases that enter a shared continuation",
    ))
}
