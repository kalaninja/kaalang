use kaalang::kaa;

#[kaa]
fn diverging_case(reading: Option<u8>) -> u8 {
    #[kaa::choice("Did the sensor answer?")]
    #[kaa::case("A reading.")]
    #[kaa::case("No reading, which the sensor rules out.")]
    let (answered, silent) = |reading| match reading {
        Some(level) => level,
        None => unreachable!("the sensor always answers"),
    };

    #[kaa::action("Use the reading.")]
    let level = |answered| answered;

    #[kaa::action("Never reached: the match arm panics first.")]
    let level = |silent| 0;

    |level| return level;
}

#[test]
fn a_case_may_diverge_in_its_arm() {
    assert_eq!(diverging_case(Some(7)), 7);
}
