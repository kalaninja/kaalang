use kaalang::kaalang;

#[kaalang]
fn blocked_terminal_crossing(request: u8) -> u8 {
    #[choice("Choose an outer path.")]
    #[case("Build the left shared value.")]
    #[case("Build the right shared value.")]
    #[case("Reach End after one step.")]
    #[case("Reach End after two steps.")]
    |request| -> (left, right, first, second) {
        match request {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Build the left shared value.")]
    |left| -> shared { 1u8 };

    #[action("Build the right shared value.")]
    |right| -> shared { 2u8 };

    #[action("Produce the first terminal result.")]
    |first| -> result { 3u8 };

    #[action("Take one more step toward the second terminal result.")]
    |second| -> stepped { 4u8 };

    #[action("Produce the second terminal result.")]
    |stepped| -> result { stepped };

    #[choice("Choose an inner path wide enough to block both terminal columns.")]
    #[case("Build A.")]
    #[case("Build B.")]
    #[case("Build C.")]
    #[case("Build D.")]
    |shared| -> (a, b, c, d) {
        match shared {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Build result A.")]
    |a| -> selected { 6u8 };

    #[action("Build result B.")]
    |b| -> selected { 7u8 };

    #[action("Build result C.")]
    |c| -> selected { 8u8 };

    #[action("Build result D.")]
    |d| -> selected { 9u8 };

    #[action("Use the result.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}

#[test]
fn reaches_end_from_every_outer_path() {
    assert_eq!(blocked_terminal_crossing(0), 7);
    assert_eq!(blocked_terminal_crossing(1), 8);
    assert_eq!(blocked_terminal_crossing(2), 3);
    assert_eq!(blocked_terminal_crossing(3), 4);
}
