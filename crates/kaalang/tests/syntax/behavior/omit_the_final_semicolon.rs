use kaalang::kaalang;

/// RFC 0001 §3: the final block statement may omit its semicolon, exactly as a
/// Rust tail expression may.
#[kaalang]
fn omit_the_final_semicolon(input: u32) -> u32 {
    #[action("Double the input.")]
    |input| -> doubled { input * 2 };

    #[action("Add one to the doubled value.")]
    |doubled| -> output { doubled + 1 };

    #[end]
    |output| {}
}

#[test]
fn a_flow_may_omit_the_final_semicolon() {
    assert_eq!(omit_the_final_semicolon(2), 5);
}
