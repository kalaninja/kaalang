use kaalang::kaalang;

/// A merge completed before a cycle with several outputs stays merged: the
/// body's routes, repeating or exporting, do not reopen the question above it.
#[kaalang]
fn merge_before_alternative_outputs(fast: bool, mut count: u32) -> u32 {
    #[question("Count fast?")]
    let (quick, slow) = |fast| fast;

    #[action("Step by two.")]
    let step = |quick| 2;

    #[action("Step by one.")]
    let step = |slow| 1;

    #[cycle("Count past five.")]
    let (even, odd) = {
        #[question("Past five?")]
        let (past, again) = |&count| *count > 5;

        #[action("Take a step.")]
        let stepped = |again, step, &mut count| *count += step;

        |stepped| continue;

        #[question("Is the count even?")]
        let (even, odd) = |past, &count| *count % 2 == 0;
    };

    #[action("Score the even count.")]
    let score = |even, count| count * 10;

    #[action("Score the odd count.")]
    let score = |odd, count| count;

    |score| return score;
}

#[test]
fn the_merged_step_drives_every_iteration() {
    assert_eq!(merge_before_alternative_outputs(true, 1), 7);
    assert_eq!(merge_before_alternative_outputs(false, 0), 60);
    assert_eq!(merge_before_alternative_outputs(true, 8), 80);
}
