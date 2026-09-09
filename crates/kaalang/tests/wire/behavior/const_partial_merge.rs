use kaalang::kaalang;

/// The join chain of a partial merge is ordinary control flow, so it remains
/// available in a `const fn`.
#[kaalang]
const fn const_partial_merge(source: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Third source.")]
    |source| -> (first, second, third) {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the value from the first source.")]
    |first| -> partial { 10u8 };

    #[action("Build the value from the second source.")]
    |second| -> partial { 20u8 };

    #[action("Add one to the partially merged value.")]
    |partial| -> shared { partial + 1 };

    #[action("Build the value from the third source.")]
    |third| -> shared { 30u8 };

    #[action("Double the merged value.")]
    |shared| -> result { shared * 2 };
}

#[test]
fn a_partial_merge_chain_evaluates_at_compile_time() {
    const FIRST: u8 = const_partial_merge(0);
    const SECOND: u8 = const_partial_merge(1);
    const THIRD: u8 = const_partial_merge(2);
    assert_eq!((FIRST, SECOND, THIRD), (22, 42, 60));
}
