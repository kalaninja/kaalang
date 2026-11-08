use kaalang::kaa;

#[kaa]
fn blocked_terminal_crossing(request: u8) -> u8 {
    #[kaa::choice("Choose an outer case.")]
    #[kaa::case("Build the left shared value.")]
    #[kaa::case("Build the right shared value.")]
    #[kaa::case("Finish after one step.")]
    #[kaa::case("Finish after two steps.")]
    let (left, right, first, second) = |request| match request {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[kaa::action("Build the left shared value.")]
    let shared = |left| 1u8;

    #[kaa::action("Build the right shared value.")]
    let shared = |right| 2;

    #[kaa::action("Produce the first terminal result.")]
    let end = |first| 3;

    #[kaa::action("Take one more step toward the second terminal result.")]
    let stepped = |second| 4;

    #[kaa::action("Produce the second terminal result.")]
    let end = |stepped| stepped;

    #[kaa::choice("Choose an inner case wide enough to block both terminal columns.")]
    #[kaa::case("Build A.")]
    #[kaa::case("Build B.")]
    #[kaa::case("Build C.")]
    #[kaa::case("Build D.")]
    let (a, b, c, d) = |shared| match shared {
        0 => (),
        1 => (),
        2 => (),
        _ => (),
    };

    #[kaa::action("Build result A.")]
    let selected = |a| 6;

    #[kaa::action("Build result B.")]
    let selected = |b| 7;

    #[kaa::action("Build result C.")]
    let selected = |c| 8;

    #[kaa::action("Build result D.")]
    let selected = |d| 9;

    #[kaa::action("Use the result.")]
    let end = |selected| selected;

    |end| return end;
}

#[test]
fn reaches_end_from_every_outer_case() {
    assert_eq!(blocked_terminal_crossing(0), 7);
    assert_eq!(blocked_terminal_crossing(1), 8);
    assert_eq!(blocked_terminal_crossing(2), 3);
    assert_eq!(blocked_terminal_crossing(3), 4);
}
