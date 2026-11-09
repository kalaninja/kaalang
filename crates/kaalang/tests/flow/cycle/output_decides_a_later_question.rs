use kaalang::kaa;

/// One output's continuation opens its own question before merging with the
/// other output's continuation.
#[kaa]
fn output_decides_a_later_question(mut values: Vec<i32>) -> i32 {
    #[kaa::cycle("Take the next positive value.")]
    let (positive, empty) = loop {
        #[kaa::choice("Is another value waiting?")]
        #[kaa::case("Check the value.")]
        #[kaa::case("The values ran out.")]
        let (value, empty) = |&mut values| match values.pop() {
            Some(value) => value,
            None => (),
        };

        #[kaa::question("Is it zero or negative?")]
        let (skip, keep) = |&value| *value <= 0;

        #[kaa::action("Keep the positive value.")]
        let positive = |keep, value| value;

        |skip| continue;
    };

    #[kaa::question("Is the positive value large?")]
    let (large, small) = |&positive| *positive > 10;

    #[kaa::action("Cap the large value.")]
    let result = |large, positive| 10;

    #[kaa::action("Keep the small value.")]
    let result = |small, positive| positive;

    #[kaa::action("Report nothing found.")]
    let result = |empty| 0;

    |result| return result;
}

#[test]
fn the_selected_output_decides_the_following_question() {
    assert_eq!(output_decides_a_later_question(vec![3, -1, 20]), 10);
    assert_eq!(output_decides_a_later_question(vec![3, -1]), 3);
    assert_eq!(output_decides_a_later_question(vec![-5]), 0);
}
