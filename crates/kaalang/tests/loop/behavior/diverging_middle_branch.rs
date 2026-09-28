use kaalang::kaalang;

/// The first and third routes meet at the iteration tail, the third and fourth
/// at the declared output, and the second never leaves its own loop. A sibling that
/// ends before either group's vertex is placed must not be held to a side of
/// that vertex: only a later sibling is.
#[kaalang]
fn diverging_middle_branch(mode: u8, stay: bool) -> u8 {
    #[cycle("Choose a repeating, diverging, or leaving route.")]
    let selected = {
        #[choice("Which route?")]
        #[case("Advance and repeat.")]
        #[case("Spin forever.")]
        #[case("Advance, then repeat or leave.")]
        #[case("Leave at once.")]
        let (advance, spin, decide, leave_now) = |mode| match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[action("Advance.")]
        let advanced = |advance| {};

        #[cycle("Spin forever.")]
        |spin| {
            #[action("Spin.")]
            || {};

            continue;
        };

        #[question("Stay in the loop?")]
        let (again, leave) = |decide, stay| stay;

        #[action("Advance after the decision.")]
        let advanced = |again| {};

        #[action("Leave after the decision.")]
        let selected = |leave, mode| mode;
        #[action("Leave immediately.")]
        let selected = |leave_now, mode| mode;
        |advanced| continue;
    };

    |selected| return selected;
}

#[test]
fn the_completing_routes_produce_the_mode() {
    assert_eq!(diverging_middle_branch(3, true), 3);
    assert_eq!(diverging_middle_branch(2, false), 2);
}
