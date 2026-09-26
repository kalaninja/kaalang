use kaalang::kaalang;

#[kaalang]
fn nested_exit_convergence(mut count: usize) -> usize {
    #[cycle("Count down to zero.")]
    let result = {
        #[cycle("Converge the inner stopping routes.")]
        {
            #[choice("Leave the inner loop?")]
            #[case("Leave at zero.")]
            #[case("Leave at one.")]
            #[case("Count down.")]
            let (zero, one, again) = |&count| match *count {
                0 => (),
                1 => (),
                _ => (),
            };

            #[action("Finish at zero.")]
            let done = |zero| {};

            #[action("Finish at one.")]
            let done = |one| {};

            |done| break;

            #[action("Count down.")]
            |again, &mut count| *count -= 1;

            |again| continue;
        };

        #[question("Finish the outer loop?")]
        let (done, again) = |&count| *count == 0;

        |done, count| break count;

        #[action("Count down once more.")]
        |again, &mut count| *count -= 1;

        |again| continue;
    };

    |result| return result;
}

#[test]
fn inner_exits_converge_before_the_outer_exit_or_repeat() {
    for count in 0..10 {
        assert_eq!(nested_exit_convergence(count), 0);
    }
}
