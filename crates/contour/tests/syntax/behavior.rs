use contour::contour;

/// RFC 0001 §3: the final block statement may omit its semicolon, exactly as a
/// Rust tail expression may.
#[contour]
fn omit_the_final_semicolon(input: u32) -> u32 {
    #[action("Double the input.")]
    |input| -> doubled { input * 2 };

    #[action("Add one to the doubled value.")]
    |doubled| -> output { doubled + 1 }
}

/// RFC 0001 §3: source comments remain ordinary Rust comments beside a block's
/// Contour attributes.
#[contour]
fn document_a_block(input: u32) -> u32 {
    /// Increments by one, because the flow needs a successor.
    #[action("Increment the input.")]
    |input| -> output { input + 1 };
}

#[test]
fn a_flow_may_omit_the_final_semicolon() {
    assert_eq!(omit_the_final_semicolon(2), 5);
}

#[test]
fn a_block_may_carry_a_doc_comment() {
    assert_eq!(document_a_block(1), 2);
}
