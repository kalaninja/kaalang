use kaalang::kaa;

#[kaa]
fn prepared_merge(seed: u32, choose: bool) -> u32 {
    #[kaa::question("Choose the prepared value.")]
    let (yes, no) = |choose| choose;

    #[kaa::action("Prepare the first value.")]
    let (shared, go) = |yes, seed| (seed + 1, ());

    #[kaa::action("Prepare the second value.")]
    let (shared, go) = |no, seed| (seed + 2, ());

    #[kaa::stage("Return the prepared value.")]
    |go| {
        |shared| return shared;
    };
}

#[test]
fn shares_a_merged_preparation_wire() {
    assert_eq!(prepared_merge(10, true), 11);
    assert_eq!(prepared_merge(10, false), 12);
}
