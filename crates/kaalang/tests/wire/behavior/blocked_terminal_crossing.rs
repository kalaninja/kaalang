use kaalang::kaalang;

#[kaalang]
fn blocked_terminal_crossing(request: u8) -> u8 {
    #[choice("Choose an outer case.")]
    #[case("Build the left shared value.")]
    #[case("Build the right shared value.")]
    #[case("Finish after one step.")]
    #[case("Finish after two steps.")]
    let (left, right, first, second) = |request| match request {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[action("Build the left shared value.")]
    let shared = |left| 1u8;

    #[action("Build the right shared value.")]
    let shared = |right| 2u8;

    #[action("Produce the first terminal result.")]
    let end = |first| 3u8;

    #[action("Take one more step toward the second terminal result.")]
    let stepped = |second| 4u8;

    #[action("Produce the second terminal result.")]
    let end = |stepped| stepped;

    #[choice("Choose an inner case wide enough to block both terminal columns.")]
    #[case("Build A.")]
    #[case("Build B.")]
    #[case("Build C.")]
    #[case("Build D.")]
    let (a, b, c, d) = |shared| match shared {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[action("Build result A.")]
    let selected = |a| 6u8;

    #[action("Build result B.")]
    let selected = |b| 7u8;

    #[action("Build result C.")]
    let selected = |c| 8u8;

    #[action("Build result D.")]
    let selected = |d| 9u8;

    #[action("Use the result.")]
    let end = |selected| selected;
}

#[test]
fn reaches_end_from_every_outer_case() {
    assert_eq!(blocked_terminal_crossing(0), 7);
    assert_eq!(blocked_terminal_crossing(1), 8);
    assert_eq!(blocked_terminal_crossing(2), 3);
    assert_eq!(blocked_terminal_crossing(3), 4);
}
