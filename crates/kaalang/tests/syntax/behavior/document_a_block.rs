use kaalang::kaa;

/// Source comments remain ordinary Rust comments beside a block's
/// kaalang attributes.
#[kaa]
fn document_a_block(input: u32) -> u32 {
    /// Increments by one, because the flow needs a successor.
    #[kaa::action("Increment the input.")]
    let end = |input| input + 1;

    |end| return end;
}

#[test]
fn a_block_may_carry_a_doc_comment() {
    assert_eq!(document_a_block(1), 2);
}
