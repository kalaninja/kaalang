use kaalang::kaa;

#[kaa]
fn is_even(mut remaining: usize) -> bool {
    #[kaa::action("Prepare the result and start at even parity.")]
    let (mut result, even) = || (false, ());

    #[kaa::stage("An even number of steps has been taken.")]
    let (finish, odd) = |even| {
        #[kaa::question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[kaa::action("Record an even result.")]
        let finish = |done, &mut result| {
            *result = true;
        };

        #[kaa::action("Take one step.")]
        let odd = |again, &mut remaining| {
            *remaining -= 1;
        };
    };

    #[kaa::stage("An odd number of steps has been taken.")]
    let (finish, even) = |odd| {
        #[kaa::question("Have all steps been taken?")]
        let (done, again) = |remaining| remaining == 0;

        #[kaa::action("Record an odd result.")]
        let finish = |done, &mut result| {
            *result = false;
        };

        #[kaa::action("Take one step.")]
        let even = |again, &mut remaining| {
            *remaining -= 1;
        };
    };

    #[kaa::stage("Return the parity.")]
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
