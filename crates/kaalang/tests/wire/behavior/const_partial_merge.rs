use kaalang::kaalang;

/// The join chain of a partial merge is ordinary control flow, so it remains
/// available in a `const fn`.
#[kaalang]
const fn const_partial_merge(source: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Third source.")]
    let (first, second, third) = |source| match source {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build the value from the first source.")]
    let partial = |first| 10;

    #[action("Build the value from the second source.")]
    let partial = |second| 20;

    #[action("Add one to the partially merged value.")]
    let shared = |partial| partial + 1;

    #[action("Build the value from the third source.")]
    let shared = |third| 30;

    #[action("Double the merged value.")]
    let end = |shared| shared * 2;

    |end| return end;
}

#[test]
fn a_partial_merge_chain_evaluates_at_compile_time() {
    const FIRST: u8 = const_partial_merge(0);
    const SECOND: u8 = const_partial_merge(1);
    const THIRD: u8 = const_partial_merge(2);
    assert_eq!((FIRST, SECOND, THIRD), (22, 42, 60));
}
