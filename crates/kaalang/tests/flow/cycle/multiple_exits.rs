use kaalang::kaa;

#[kaa]
fn multiple_exits(mut mode: u8) -> u8 {
    #[kaa::cycle("Select an exit after zero or one advances.")]
    let result = loop {
        #[kaa::choice("Exit or advance?")]
        #[kaa::case("Exit immediately.")]
        #[kaa::case("Exit after advancing.")]
        #[kaa::case("Advance once.")]
        let (first, last, advance) = |mode| match mode {
            0 => (),
            2 => (),
            _ => (),
        };

        #[kaa::action("Keep the immediate result.")]
        let result = |first, mode| mode;

        #[kaa::action("Advance to the final case.")]
        |advance, &mut mode| *mode = 2;

        #[kaa::action("Keep the result after advancing.")]
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
