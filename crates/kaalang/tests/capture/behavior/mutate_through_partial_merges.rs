use kaalang::kaalang;

#[kaalang]
fn mutate_through_partial_merges(source: u8, refine: bool) -> Vec<u8> {
    #[choice("Choose the source.")]
    #[case("Refined source.")]
    #[case("Second source.")]
    #[case("Fallback.")]
    |source| -> (first, second, third) {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[question("Refine the first source?")]
    |first, refine| -> (yes, no) { refine };

    #[action("Build the refined value.")]
    |yes| -> partial { vec![1] };

    #[action("Build the unrefined value.")]
    |no| -> partial { vec![2] };

    #[action("Build the second value.")]
    |second| -> partial { vec![3] };

    #[action("Extend the partially merged value.")]
    |&mut partial| {
        partial.push(4);
    };

    #[action("Carry the partial value to the outer merge.")]
    |partial| -> shared { partial };

    #[action("Build the fallback value.")]
    |third| -> shared { vec![0] };

    #[action("Extend the fully merged value.")]
    |&mut shared| {
        shared.push(5);
    };

    #[action("Return the changed value.")]
    |shared| -> result { shared };
}

#[test]
fn nested_and_partial_merges_bind_mutable_storage() {
    for (source, refine, expected) in [
        (0, true, vec![1, 4, 5]),
        (0, false, vec![2, 4, 5]),
        (1, false, vec![3, 4, 5]),
        (2, false, vec![0, 5]),
    ] {
        assert_eq!(mutate_through_partial_merges(source, refine), expected);
    }
}
