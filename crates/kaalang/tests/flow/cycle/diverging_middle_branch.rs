use kaalang::kaa;

/// The first and third routes meet at the iteration tail, the third and fourth
/// at the declared output, and the second never leaves its own cycle. A sibling that
/// ends before either group's vertex is placed must not be held to a side of
/// that vertex: only a later sibling is.
#[kaa]
fn diverging_middle_branch(mode: u8, stay: bool) -> u8 {
    #[kaa::cycle("Choose a repeating, diverging, or leaving route.")]
    let selected = loop {
        #[kaa::choice("Which route?")]
        #[kaa::case("Advance and repeat.")]
        #[kaa::case("Spin forever.")]
        #[kaa::case("Advance, then repeat or leave.")]
        #[kaa::case("Leave at once.")]
        let (advance, spin, decide, leave_now) = |mode| match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[kaa::action("Advance.")]
        let advanced = |advance| {};

        #[kaa::cycle("Spin forever.")]
        |spin| loop {
            #[kaa::action("Spin.")]
            || {};

            continue;
        };

        #[kaa::question("Stay in the cycle?")]
        let (again, leave) = |decide, stay| stay;

        #[kaa::action("Advance after the decision.")]
        let advanced = |again| {};

        #[kaa::action("Leave after the decision.")]
        let selected = |leave, mode| mode;
        #[kaa::action("Leave immediately.")]
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
