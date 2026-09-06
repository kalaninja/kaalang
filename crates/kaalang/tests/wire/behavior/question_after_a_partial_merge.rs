use kaalang::kaalang;

#[kaalang]
fn question_after_a_partial_merge(value: u8, flag: bool) -> u8 {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Third source.")]
    |value| -> (first, second, third) {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the value from the first source.")]
    |first| -> partial { 10u8 };

    #[action("Build the value from the second source.")]
    |second| -> partial { 20u8 };

    #[action("Use the partially merged value.")]
    |partial| -> shared { partial + 1 };

    #[action("Build the value from the third source.")]
    |third| -> shared { 30u8 };

    #[question("Is the value large?")]
    |&shared, flag| -> (large, small) { flag && *shared > 15 };

    #[action("Double a large value.")]
    |large, shared| -> result { shared * 2 };

    #[action("Keep a small value.")]
    |small, shared| -> result { shared };
}

#[test]
fn a_question_of_the_wider_group_may_follow_a_partial_merge() {
    for (value, flag, expected) in [
        (0, true, 11),
        (1, true, 42),
        (2, true, 60),
        (0, false, 11),
        (1, false, 21),
        (2, false, 30),
    ] {
        assert_eq!(question_after_a_partial_merge(value, flag), expected);
    }
}
