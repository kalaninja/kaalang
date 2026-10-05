use kaalang::kaalang;

#[kaalang]
fn prepared_cycle(seed: u32) -> u32 {
    #[cycle("Produce shared data.")]
    let mut shared = loop {
        #[action("Compute the result.")]
        let shared = |seed| seed + 1;
    };

    #[action("Enter the stage.")]
    let go = || {};

    #[stage("Return the prepared result.")]
    |go| {
        #[action("Increment the shared result.")]
        |&mut shared| *shared += 1;

        |shared| return shared;
    };
}

#[test]
fn a_cycle_result_remains_available_to_a_stage() {
    assert_eq!(prepared_cycle(7), 9);
}
