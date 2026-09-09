use kaalang::kaalang;

/// Raw and plain spellings name one wire: at a keyword flow input, at a block
/// input of a plain parameter, at a keyword output, at an ignored output, and
/// at the `result` wire itself.
#[kaalang]
fn raw_spellings(r#type: u8, value: u8) -> u8 {
    #[action("Combine the keyword-named input with the plain one.")]
    let (r#match, r#_ignored) = |r#type, r#value| (r#type + value, ());

    #[action("Preserve the renamed value.")]
    let r#result = |r#match| r#match;
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_spellings(3, 4), 7);
}
