use kaalang::kaalang;

#[kaalang]
fn is_even(mut remaining: usize) -> bool {
    #[action("Prepare the result and start at even parity.")]
    let (mut result, even) = || (false, ());

    #[stage("An even number of steps has been taken.")]
    let (finish, odd) = |even| {
        #[question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[action("Record an even result.")]
        let finish = |done, &mut result| {
            *result = true;
        };

        #[action("Take one step.")]
        let odd = |again, &mut remaining| {
            *remaining -= 1;
        };
    };

    #[stage("An odd number of steps has been taken.")]
    let (finish, even) = |odd| {
        #[question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[action("Record an odd result.")]
        let finish = |done, &mut result| {
            *result = false;
        };

        #[action("Take one step.")]
        let even = |again, &mut remaining| {
            *remaining -= 1;
        };
    };

    #[stage("Return the parity.")]
    |finish| {
        |result| return result;
    };
}

#[test]
fn alternates_visits_before_the_terminal_stage() {
    for count in 0..8 {
        assert_eq!(is_even(count), count % 2 == 0);
    }
}
