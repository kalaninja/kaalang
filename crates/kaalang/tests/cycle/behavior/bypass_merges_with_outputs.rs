use kaalang::kaalang;

#[kaalang]
fn bypass_merges_with_outputs(cached: bool, response: u8, mut retries: u8) -> u8 {
    #[question("Is the record cached?")]
    let (hit_cache, query) = |cached| cached;

    #[cycle("Query the source until it answers or the retries run out.")]
    let (found, missing, gave_up) = |query| loop {
        #[choice("What did the source answer?")]
        #[case("The record.")]
        #[case("No such record.")]
        #[case("A busy signal.")]
        let (found, missing, busy) = |response| match response {
            0 => (),
            1 => (),
            _ => (),
        };

        #[question("Have the retries run out?")]
        let (gave_up, retry) = |busy, &retries| *retries == 0;

        #[action("Spend one retry.")]
        let spent = |retry, &mut retries| *retries -= 1;

        |spent| continue;
    };

    #[action("Use the cached record.")]
    let record = |hit_cache| 10;

    #[action("Use the fetched record.")]
    let record = |found| 20;

    #[action("Use an empty record.")]
    let record = |missing| 30;

    #[action("Format the record.")]
    let answer = |record| record + 1;

    #[action("Report that the source gave up.")]
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
