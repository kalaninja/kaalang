//! A Cargo target, ordered as RFC 0009 §4.3 orders the cards of its types.

use std::path::Path;

/// A target of a package: its library, a binary, a test, an example, or a
/// bench, which Rust compiles as a crate of its own.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CargoTarget<'a> {
    pub package: &'a str,
    /// Library and binaries, tests, examples, then benches.
    pub group: u8,
    pub name: &'a str,
    pub root: &'a Path,
}

impl<'a> CargoTarget<'a> {
    /// The focus's crate comes first, then the other targets of its package.
    #[must_use]
    pub fn order(self, focus_package: Option<&str>, is_focus: bool) -> (bool, bool, Self) {
        (!is_focus, Some(self.package) != focus_package, self)
    }

    /// Each package's target groups start separate rows in the diagram.
    #[must_use]
    pub fn group_key(self) -> (&'a str, u8) {
        (self.package, self.group)
    }
}
