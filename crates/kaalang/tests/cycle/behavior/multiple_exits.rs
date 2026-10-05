use kaalang::kaalang;

#[kaalang]
fn multiple_exits(mut mode: u8) -> u8 {
    #[cycle("Select an exit after zero or one advances.")]
    let result = loop {
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

        |advance| continue;
    };

    |result| return result;
}

#[test]
fn exit_routes_merge_before_the_single_output() {
    assert_eq!(multiple_exits(0), 0);
    assert_eq!(multiple_exits(1), 2);
    assert_eq!(multiple_exits(2), 2);
}
