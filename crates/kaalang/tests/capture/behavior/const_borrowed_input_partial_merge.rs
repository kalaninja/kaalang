use kaalang::kaalang;

/// Borrowed input data outlives both the partial and wider joins, including
/// when the flow runs during constant evaluation.
#[kaalang]
const fn const_borrowed_input_partial_merge(source: u8, bytes: &[u8]) -> usize {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Fallback.")]
    |source| -> (first, second, fallback) {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Use the input on the first branch.")]
    |first, bytes| -> partial { bytes };

    #[action("Use the input on the second branch.")]
    |second, bytes| -> partial { bytes };

    #[action("Carry the partially merged slice onward.")]
    |partial| -> view { partial };

    #[action("Use the fallback slice.")]
    |fallback| -> view { &[7u8, 8] };

    #[action("Measure the merged slice.")]
    |view| -> result { view.len() + view[0] as usize };
}

#[test]
fn borrowed_input_crosses_the_partial_merge_at_compile_time() {
    const FIRST: usize = const_borrowed_input_partial_merge(0, &[1u8, 2, 3]);
    const SECOND: usize = const_borrowed_input_partial_merge(1, &[4u8, 5, 6]);
    const FALLBACK: usize = const_borrowed_input_partial_merge(2, &[]);
    assert_eq!((FIRST, SECOND, FALLBACK), (4, 7, 9));
}
