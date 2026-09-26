use kaalang::kaalang;

#[kaalang]
fn cycle_on_each_branch(up: bool, mut n: u32) -> u32 {
    #[question("Count up?")]
    let (rise, fall) = |up| up;

    #[cycle("Count up to ten.")]
    let end = |rise| {
        #[question("Reached ten?")]
        let (done, more) = |&n| *n >= 10;

        |done, n| break n;

        #[action("Step up.")]
        let stepped = |more, &mut n| *n += 1;

        |stepped| continue;
    };

    #[cycle("Count down to zero.")]
    let end = |fall| {
        #[question("Reached zero?")]
        let (done, more) = |&n| *n == 0;

        |done, n| break n;

        #[action("Step down.")]
        let stepped = |more, &mut n| *n -= 1;

        |stepped| continue;
    };

    |end| return end;
}

#[test]
fn each_branch_repeats_its_own_cycle_before_the_merge() {
    assert_eq!(cycle_on_each_branch(true, 3), 10);
    assert_eq!(cycle_on_each_branch(false, 3), 0);
}
