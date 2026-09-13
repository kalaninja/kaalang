use kaalang::kaalang;

/// The first and third routes meet at the iteration tail, the third and fourth
/// after the loop, and the second never leaves its own loop. A sibling that
/// ends before either group's vertex is placed must not be held to a side of
/// that vertex: only a later sibling is (RFC 0002 §8).
#[kaalang]
fn diverging_middle_branch(mode: u8, stay: bool) -> u8 {
    loop {
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
        |advance| {};

        |spin| loop {
            #[action("Spin.")]
            || {};
        };

        #[question("Stay in the loop?")]
        let (again, leave) = |decide, stay| stay;

        #[action("Advance after the decision.")]
        |again| {};

        |leave| break;
        |leave_now| break;
    }

    #[action("Return the mode.")]
    let end = |mode| mode;
}

#[test]
fn the_leaving_routes_return_the_mode() {
    assert_eq!(diverging_middle_branch(3, true), 3);
    assert_eq!(diverging_middle_branch(2, false), 2);
}
