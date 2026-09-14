use kaalang::kaalang;

#[kaalang]
fn multiple_exits(mode: u8) -> u8 {
    #[cycle("Select an exit after zero or one advances.")]
    let selected = |mut mode| {
        #[choice("Exit or advance?")]
        #[case("Exit immediately.")]
        #[case("Exit after advancing.")]
        #[case("Advance once.")]
        let (first, last, advance) = |mode| match mode {
            0 => (),
            2 => (),
            _ => (),
        };

        #[action("Keep the immediate result.")]
        let result = |first, mode| mode;

        #[action("Advance to the final case.")]
        |advance, &mut mode| *mode = 2;

        #[action("Keep the result after advancing.")]
        let result = |last, mode| mode;

        |result| break result;
    };

    |selected| return selected;
}

#[test]
fn exit_routes_merge_before_the_single_break() {
    assert_eq!(multiple_exits(0), 0);
    assert_eq!(multiple_exits(1), 2);
    assert_eq!(multiple_exits(2), 2);
}
