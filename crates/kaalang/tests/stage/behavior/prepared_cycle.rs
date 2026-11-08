use kaalang::kaa;

#[kaa]
fn prepared_cycle(seed: u32) -> u32 {
    #[kaa::cycle("Produce shared data.")]
    let mut shared = loop {
        #[kaa::action("Compute the result.")]
        let shared = |seed| seed + 1;
    };

    #[kaa::action("Enter the stage.")]
    let go = || {};

    #[kaa::stage("Return the prepared result.")]
    |go| {
        #[kaa::action("Increment the shared result.")]
        |&mut shared| *shared += 1;

        |shared| return shared;
    };
}

#[test]
fn a_cycle_result_remains_available_to_a_stage() {
    assert_eq!(prepared_cycle(7), 9);
}
