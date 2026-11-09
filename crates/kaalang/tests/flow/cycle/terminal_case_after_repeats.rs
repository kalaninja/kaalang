use kaalang::kaa;

#[kaa]
fn terminal_case_after_repeats(mut mode: u8) -> u8 {
    #[kaa::cycle("Repeat until the finishing case.")]
    let result = loop {
        #[kaa::choice("Which route?")]
        #[kaa::case("Advance on the left.")]
        #[kaa::case("Advance on the right.")]
        #[kaa::case("Finish after the repeating cases.")]
        let (left, right, finish) = |mode| match mode {
            0 => (),
            2.. => (),
            _ => (),
        };

        #[kaa::action("Finish from the final case.")]
        let result = |finish| 7;

        #[kaa::action("Advance through the left case.")]
        let advanced = |left, &mut mode| *mode = 1;

        #[kaa::action("Advance through the right case.")]
        let advanced = |right, &mut mode| *mode = 1;

        |advanced| continue;
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
