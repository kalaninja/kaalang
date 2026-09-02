use kaalang::kaalang;

#[kaalang]
fn convergence_at_end(case: u8) -> u32 {
    #[choice("Choose whether to continue.")]
    #[case("First continuing path")]
    #[case("Second continuing path")]
    #[case("Direct End path")]
    |case| -> (first, second, done) {
        match case {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { 1 };

    #[action("Build the second value.")]
    |second| -> selected { 2 };

    #[action("Produce the direct result.")]
    |done| -> result { 99 };

    #[action("Use a value from a continuing path.")]
    |selected| -> result { selected * 10 };

    #[end]
    |result| {};
}

#[test]
fn alternative_paths_converge_at_end() {
    assert_eq!(convergence_at_end(0), 10);
    assert_eq!(convergence_at_end(1), 20);
    assert_eq!(convergence_at_end(2), 99);
}
