use kaalang::kaalang;

/// RFC 0001 §3: source comments remain ordinary Rust comments beside a block's
/// kaalang attributes.
#[kaalang]
fn document_a_block(input: u32) -> u32 {
    /// Increments by one, because the flow needs a successor.
    #[action("Increment the input.")]
    let result = |input| input + 1;
}

#[test]
fn a_block_may_carry_a_doc_comment() {
    assert_eq!(document_a_block(1), 2);
}
