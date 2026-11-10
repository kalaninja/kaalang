//! The cfg set of one compilation target, which decides the `cfg`s of the
//! source a diagram reads.

use std::str::FromStr;

use cargo_platform::{Cfg, CfgExpr};
use proc_macro2::TokenStream;

/// The cfg set of one compilation target, as `rustc --print cfg` prints it:
/// one `name` or `key="value"` per line, such as `unix` or `target_os="linux"`,
/// with a `feature="…"` line for each enabled feature. A diagram reads its
/// source as that target compiles it. The default set is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Target {
    cfgs: Vec<Cfg>,
}

impl Target {
    /// The empty set, for options that name no target.
    pub const EMPTY: &'static Self = &Self { cfgs: Vec::new() };

    /// Whether `predicate`, the tokens inside `cfg(...)`, holds for the set,
    /// as cargo decides one: a name the set lacks does not hold, nor does a
    /// predicate cargo cannot read.
    // shortcut: cargo cannot read a raw string such as `feature = r"audit"`,
    // which rustc accepts, so it does not hold; unescape literals if one shows up.
    #[must_use]
    pub fn holds(&self, predicate: &TokenStream) -> bool {
        CfgExpr::from_str(&predicate.to_string())
            .is_ok_and(|expression| expression.matches(&self.cfgs))
    }

    /// Removes from `file` what does not hold for the set, as rustc strips a
    /// crate for its target.
    pub fn configure(&self, file: &mut syn::File) {
        kaalang_compiler::configure(file, &mut |predicate| self.holds(predicate));
    }
}

impl FromStr for Target {
    type Err = String;

    /// Reads one cfg from each nonempty line.
    fn from_str(lines: &str) -> Result<Self, Self::Err> {
        lines
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| Cfg::from_str(line).map_err(|error| format!("`{line}`: {error}")))
            .collect::<Result<_, _>>()
            .map(|cfgs| Self { cfgs })
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::Target;

    #[test]
    fn a_predicate_holds_as_cargo_decides_it() {
        let target: Target = "unix\ntarget_os=\"linux\"\nfeature=\"audit\"\n"
            .parse()
            .unwrap();
        for predicate in [
            quote!(unix),
            quote!(r#unix),
            quote!(target_os = "linux"),
            quote!(all(unix, feature = "audit")),
            quote!(not(windows)),
            quote!(any(windows, unix)),
            quote!(all()),
            quote!(true),
        ] {
            assert!(target.holds(&predicate), "{predicate}");
        }
        for predicate in [
            quote!(windows),
            quote!(test),
            quote!(target_os = "macos"),
            quote!(any()),
            quote!(false),
        ] {
            assert!(!target.holds(&predicate), "{predicate}");
        }
    }

    #[test]
    fn a_predicate_cargo_cannot_read_does_not_hold() {
        assert!(!Target::EMPTY.holds(&quote!(version("1.80"))));
    }

    #[test]
    fn a_line_that_is_not_a_cfg_is_rejected() {
        assert!("unix\nnot a cfg".parse::<Target>().is_err());
    }
}
