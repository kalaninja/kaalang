use kaalang::kaalang;

#[kaalang]
fn extra_step_before_the_tail(mut n: u32) -> u32 {
    #[cycle("Climb past ten.")]
    let result = {
        #[question("Is the number past ten?")]
        let (leave, rest) = |&n| *n > 10;

        |leave, n| break n;

        #[question("Step, then nest on an even number?")]
        let (nest, again) = |rest, &mut n| {
            *n += 1;
            *n % 2 == 0
        };

        #[cycle("Climb to a multiple of three.")]
        let again = |nest| {
            #[question("Is it a multiple of three?")]
            let (done, more) = |&n| *n % 3 == 0;

            |done| break;

            #[action("Take one step.")]
            let stepped = |more, &mut n| *n += 1;

            |stepped| continue;
        };

        #[action("Take an extra step.")]
        let bumped = |again, &mut n| *n += 1;

        |bumped| continue;
    };

    |result| return result;
}

#[test]
fn both_repeating_routes_take_the_extra_step() {
    assert_eq!(extra_step_before_the_tail(11), 11);
    assert_eq!(extra_step_before_the_tail(10), 12);
    assert_eq!(extra_step_before_the_tail(9), 13);
}
