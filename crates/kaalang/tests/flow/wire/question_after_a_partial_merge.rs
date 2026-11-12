use kaalang::kaa;

#[kaa]
fn question_after_a_partial_merge(value: u8, flag: bool) -> u8 {
    #[kaa::choice("Which source?")]
    #[kaa::case("First source.")]
    #[kaa::case("Second source.")]
    #[kaa::case("Third source.")]
    let (first, second, third) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Build the value from the first source.")]
    let partial = |first| 10;

    #[kaa::action("Build the value from the second source.")]
    let partial = |second| 20;

    #[kaa::action("Use the partially merged value.")]
    let shared = |partial| partial + 1;

    #[kaa::action("Build the value from the third source.")]
    let shared = |third| 30;

    #[kaa::question("Is the value large?")]
    let (large, small) = |&shared, flag| flag && *shared > 15;

    #[kaa::action("Double a large value.")]
    let end = |large, shared| shared * 2;

    #[kaa::action("Keep a small value.")]
    let end = |small, shared| shared;

    |end| return end;
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
