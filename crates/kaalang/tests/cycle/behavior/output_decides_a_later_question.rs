use kaalang::kaalang;

/// One output's continuation opens its own question before merging with the
/// other output's continuation.
#[kaalang]
fn output_decides_a_later_question(mut values: Vec<i32>) -> i32 {
    #[cycle("Take the next positive value.")]
    let (positive, empty) = {
        #[choice("Is another value waiting?")]
        #[case("Check the value.")]
        #[case("The values ran out.")]
        let (value, empty) = |&mut values| match values.pop() {
            Some(value) => value,
            None => (),
        };

        #[question("Is it zero or negative?")]
        let (skip, keep) = |&value| *value <= 0;

        #[action("Keep the positive value.")]
        let positive = |keep, value| value;

        |skip| continue;
    };

    #[question("Is the positive value large?")]
    let (large, small) = |&positive| *positive > 10;

    #[action("Cap the large value.")]
    let result = |large, positive| 10;

    #[action("Keep the small value.")]
    let result = |small, positive| positive;

    #[action("Report nothing found.")]
    let result = |empty| 0;

    |result| return result;
}

#[test]
fn the_selected_output_decides_the_following_question() {
    assert_eq!(output_decides_a_later_question(vec![3, -1, 20]), 10);
    assert_eq!(output_decides_a_later_question(vec![3, -1]), 3);
    assert_eq!(output_decides_a_later_question(vec![-5]), 0);
}
