use kaalang::kaalang;

/// An inner cycle exports `larger` on two routes, and a question after it
/// decides the outer cycle's outputs. The inner route orders nothing in the
/// outer body.
#[kaalang]
fn inner_routes_before_a_question(value: u8, flag: u8) -> u8 {
    #[cycle("Classify the value.")]
    let (first, second) = {
        #[cycle("Sort the value.")]
        let (tiny, larger) = {
            #[question("Is it below ten?")]
            let (low, high) = |value| value < 10;

            #[question("Is it below five?")]
            let (below, small) = |low, value| value < 5;

            #[action("Mark a tiny value.")]
            let tiny = |below| {};

            #[action("Mark a small value.")]
            let larger = |small| {};

            #[action("Mark a high value.")]
            let larger = |high| {};
        };

        #[action("Take the first class for a tiny value.")]
        let first = |tiny| 1;

        #[question("Is the flag even?")]
        let (even, odd) = |larger, flag| flag.is_multiple_of(2);

        #[action("Take the first class for an even flag.")]
        let first = |even| 2;

        #[action("Take the second class.")]
        let second = |odd| 3;
    };

    #[action("Keep the first class.")]
    let result = |first| first;

    #[action("Scale the second class.")]
    let result = |second| second * 10;

    |result| return result;
}

#[test]
fn the_inner_routes_meet_one_outer_question() {
    assert_eq!(inner_routes_before_a_question(3, 1), 1);
    assert_eq!(inner_routes_before_a_question(12, 0), 2);
    assert_eq!(inner_routes_before_a_question(7, 1), 30);
}
