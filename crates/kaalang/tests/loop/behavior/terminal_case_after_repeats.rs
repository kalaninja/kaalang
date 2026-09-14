use kaalang::kaalang;

#[kaalang]
fn terminal_case_after_repeats(mode: u8) -> u8 {
    #[cycle("Repeat until the finishing case.")]
    let result = |mut mode| {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Advance on the right.")]
        #[case("Finish after the repeating cases.")]
        let (left, right, finish) = |mode| match mode {
            0 => (),
            2.. => (),
            _ => (),
        };

        #[action("Finish from the final case.")]
        let result = |finish| 7;

        |result| break result;

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    };

    |result| return result;
}

/// Finishing after both repeating cases leaves end below the iteration back edge.
#[test]
fn a_finishing_case_after_the_repeats_keeps_its_result() {
    assert_eq!(terminal_case_after_repeats(0), 7);
    assert_eq!(terminal_case_after_repeats(1), 7);
    assert_eq!(terminal_case_after_repeats(2), 7);
}
