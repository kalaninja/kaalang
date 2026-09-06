use kaalang::kaalang;

/// RFC 0001 §3: macro token streams are opaque to the control-transfer check,
/// so a macro may mention `return` without transferring control.
#[kaalang]
fn keep_macro_tokens_opaque() -> &'static str {
    #[action("Spell the keyword instead of using it.")]
    || -> result { stringify!(return) };
}

#[test]
fn a_macro_may_mention_return_without_transferring_control() {
    assert_eq!(keep_macro_tokens_opaque(), "return");
}
