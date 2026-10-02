use kaalang::kaalang;

/// Two routes export one output of a cycle entered after a merge. They differ
/// only inside the body, so the merge above the cycle stays closed.
#[kaalang]
fn output_routes_after_a_merge(double: bool, mode: u8) -> u32 {
    #[question("Double the score?")]
    let (twice, once) = |double| double;

    #[action("Use a factor of two.")]
    let factor = |twice| 2;

    #[action("Use a factor of one.")]
    let factor = |once| 1;

    #[cycle("Grade the mode.")]
    let (pass, fail) = {
        #[choice("Which grade?")]
        #[case("Top marks.")]
        #[case("Enough marks.")]
        #[case("Too few marks.")]
        let (top, enough, fail) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Pass with top marks.")]
        let pass = |top| 10;

        #[action("Pass with enough marks.")]
        let pass = |enough| 5;
    };

    #[action("Score the pass.")]
    let score = |pass, factor| pass * factor;

    #[action("Score the failure.")]
    let score = |fail| 0;

    |score| return score;
}

#[test]
fn both_passing_routes_meet_the_merged_factor() {
    assert_eq!(output_routes_after_a_merge(true, 0), 20);
    assert_eq!(output_routes_after_a_merge(false, 1), 5);
    assert_eq!(output_routes_after_a_merge(true, 2), 0);
}
