use kaalang::kaalang;

#[kaalang]
fn inner_cycle_joins_the_tail(mut n: u32) -> u32 {
    #[cycle("Climb past ten.")]
    let leave = {
        #[question("Is the number past ten?")]
        let (leave, rest) = |&n| *n > 10;

        #[question("Step, then nest on an even number?")]
        let (nest, again) = |rest, &mut n| {
            *n += 1;
            *n % 2 == 0
        };

        #[cycle("Climb to a multiple of three.")]
        let again = |nest| {
            #[question("Is it a multiple of three?")]
            let (again, more) = |&n| *n % 3 == 0;

            #[action("Take one step.")]
            let stepped = |more, &mut n| *n += 1;

            |stepped| continue;
        };

        |again| continue;
    };

    |leave, n| return n;
}

#[test]
fn a_branch_and_an_inner_cycle_repeat_through_one_merge() {
    // Leaves at once.
    assert_eq!(inner_cycle_joins_the_tail(11), 11);
    // An odd step repeats straight through `again`.
    assert_eq!(inner_cycle_joins_the_tail(10), 11);
    // 10 is even: the inner cycle steps to 12, then the outer one leaves.
    assert_eq!(inner_cycle_joins_the_tail(9), 12);
}
