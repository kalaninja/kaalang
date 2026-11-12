use kaalang::kaa;

#[kaa]
fn bypass_merges_with_outputs(cached: bool, response: u8, mut retries: u8) -> u8 {
    #[kaa::question("Is the record cached?")]
    let (hit_cache, query) = |cached| cached;

    #[kaa::cycle("Query the source until it answers or the retries run out.")]
    let (found, missing, gave_up) = |query| loop {
        #[kaa::choice("What did the source answer?")]
        #[kaa::case("The record.")]
        #[kaa::case("No such record.")]
        #[kaa::case("A busy signal.")]
        let (found, missing, busy) = |response| match response {
            0 => (),
            1 => (),
            _ => (),
        };

        #[kaa::question("Have the retries run out?")]
        let (gave_up, retry) = |busy, &retries| *retries == 0;

        #[kaa::action("Spend one retry.")]
        let spent = |retry, &mut retries| *retries -= 1;

        |spent| continue;
    };

    #[kaa::action("Use the cached record.")]
    let record = |hit_cache| 10;

    #[kaa::action("Use the fetched record.")]
    let record = |found| 20;

    #[kaa::action("Use an empty record.")]
    let record = |missing| 30;

    #[kaa::action("Format the record.")]
    let answer = |record| record + 1;

    #[kaa::action("Report that the source gave up.")]
    let answer = |gave_up| 0;

    |answer| return answer;
}

#[test]
fn a_bypass_merges_with_two_of_the_cycle_outputs() {
    assert_eq!(bypass_merges_with_outputs(true, 2, 0), 11);
    assert_eq!(bypass_merges_with_outputs(false, 0, 0), 21);
    assert_eq!(bypass_merges_with_outputs(false, 1, 0), 31);
    assert_eq!(bypass_merges_with_outputs(false, 2, 2), 0);
}
