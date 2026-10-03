use kaalang::kaalang;

#[kaalang]
fn for_inside_loop(mut rounds: u32, values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Repeat the rounds.")]
    let finished = loop {
        #[question("Is another round left?")]
        let (round, finished) = |&rounds| *rounds > 0;

        #[cycle("Add every value.")]
        let added = |round, values| {
            for value in values {
                #[action("Add the value.")]
                |value, &mut total| *total += *value;
            }
        };

        #[action("Count the round.")]
        let counted = |added, &mut rounds| *rounds -= 1;

        |counted| continue;
    };

    |finished, total| return total;
}

#[test]
fn a_for_cycle_runs_in_full_on_every_outer_iteration() {
    assert_eq!(for_inside_loop(0, &[1, 2]), 0);
    assert_eq!(for_inside_loop(3, &[1, 2]), 9);
}
