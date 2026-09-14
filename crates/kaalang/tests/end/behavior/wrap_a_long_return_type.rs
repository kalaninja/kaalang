use kaalang::kaalang;

/// The end node is captioned with the return type, so a long one has to wrap
/// inside the node like any other label.
#[kaalang]
fn wrap_a_long_return_type(
    input: u8,
) -> Result<Vec<(u8, String)>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    #[action("Pair the input with an empty name.")]
    let end = |input| Ok(vec![(input, String::new())]);

    |end| return end;
}

#[test]
fn a_long_return_type_still_names_the_contract() {
    assert_eq!(wrap_a_long_return_type(7).unwrap(), [(7, String::new())]);
}
