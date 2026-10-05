use kaalang::kaalang;

#[kaalang]
fn nested_exit_convergence(mut count: usize) -> usize {
    #[cycle("Count down to zero.")]
    let done = loop {
        #[cycle("Converge the inner stopping routes.")]
        let inner_done = loop {
            #[choice("Leave the inner cycle?")]
            #[case("Leave at zero.")]
            #[case("Leave at one.")]
            #[case("Count down.")]
            let (zero, one, again) = |&count| match *count {
                0 => (),
                1 => (),
                _ => (),
            };

            #[action("Finish at zero.")]
            let inner_done = |zero| {};

            #[action("Finish at one.")]
            let inner_done = |one| {};

            #[action("Count down.")]
            |again, &mut count| *count -= 1;

            |again| continue;
        };

        #[question("Finish the outer cycle?")]
        let (done, again) = |inner_done, &count| *count == 0;

        #[action("Count down once more.")]
        |again, &mut count| *count -= 1;

        |again| continue;
    };

    |done, count| return count;
}

#[test]
fn inner_exits_converge_before_the_outer_exit_or_repeat() {
    for count in 0..10 {
        assert_eq!(nested_exit_convergence(count), 0);
    }
}
