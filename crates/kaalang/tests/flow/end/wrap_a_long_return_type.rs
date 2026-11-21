use kaalang::kaa;

/// A long return type wraps inside the end node like any other label.
#[kaa]
fn wrap_a_long_return_type(
    input: u8,
) -> Result<Vec<(u8, String)>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    #[kaa::action("Pair the input with an empty name.")]
    let end = |input| Ok(vec![(input, String::new())]);

    |end| return end;
}

#[test]
fn a_long_return_type_still_names_the_contract() {
    assert_eq!(wrap_a_long_return_type(7).unwrap(), [(7, String::new())]);
}
