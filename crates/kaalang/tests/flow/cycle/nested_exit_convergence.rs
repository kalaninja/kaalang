use kaalang::kaa;

#[kaa]
fn nested_exit_convergence(mut count: usize) -> usize {
    #[kaa::cycle("Count down to zero.")]
    let done = loop {
        #[kaa::cycle("Converge the inner stopping routes.")]
        let inner_done = loop {
            #[kaa::choice("Leave the inner cycle?")]
            #[kaa::case("Leave at zero.")]
            #[kaa::case("Leave at one.")]
            #[kaa::case("Count down.")]
            let (zero, one, again) = |&count| match *count {
                0 => (),
                1 => (),
                _ => (),
            };

            #[kaa::action("Finish at zero.")]
            let inner_done = |zero| {};

            #[kaa::action("Finish at one.")]
            let inner_done = |one| {};

            #[kaa::action("Count down.")]
            |again, &mut count| *count -= 1;

            |again| continue;
        };

        #[kaa::question("Finish the outer cycle?")]
        let (done, again) = |inner_done, &count| *count == 0;

        #[kaa::action("Count down once more.")]
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
