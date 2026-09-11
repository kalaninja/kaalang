use kaalang::kaalang;

#[kaalang]
fn multiple_exits(mut mode: u8) -> u8 {
    loop {
        #[choice("Exit or advance?")]
        #[case("Exit immediately.")]
        #[case("Exit after advancing.")]
        #[case("Advance once.")]
        let (first, last, advance) = |mode| match mode {
            0 => (),
            2 => (),
            _ => (),
        };

        |first| break;

        #[action("Advance to the final case.")]
        |advance, &mut mode| *mode = 2;

        |last| break;
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

#[test]
fn distinct_break_routes_share_one_after_loop_continuation() {
    assert_eq!(multiple_exits(0), 0);
    assert_eq!(multiple_exits(1), 2);
    assert_eq!(multiple_exits(2), 2);
}
