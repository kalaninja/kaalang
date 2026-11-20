//! A Cargo target, ordered as RFC 0009 §4.3 orders the cards of its types.

use std::path::Path;

/// The group whose row a Cargo target's cards start, by the `kinds` Cargo
/// gives it: its library and binaries 0, a test 1, an example 2, a bench 3. A
/// build script has none: it draws no cards.
#[must_use]
pub fn target_group(kinds: &[impl AsRef<str>]) -> Option<u8> {
    let is = |kind: &str| kinds.iter().any(|given| given.as_ref() == kind);
    if is("custom-build") {
        None
    } else if is("test") {
        Some(1)
    } else if is("example") {
        Some(2)
    } else if is("bench") {
        Some(3)
    } else {
        Some(0)
    }
}

/// The name of the crate rustc compiles a Cargo target as.
#[must_use]
pub fn crate_name(target: &str) -> String {
    target.replace('-', "_")
}

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

#[cfg(test)]
mod tests {
    use super::{crate_name, target_group};

    #[test]
    fn a_target_kind_gives_its_row_group_and_a_build_script_none() {
        let groups = ["lib", "bin", "test", "example", "bench", "custom-build"]
            .map(|kind| target_group(&[kind]));
        assert_eq!(groups, [Some(0), Some(0), Some(1), Some(2), Some(3), None]);
        assert_eq!(target_group(&["cdylib", "rlib"]), Some(0));
        assert_eq!(crate_name("check-model"), "check_model");
    }
}
