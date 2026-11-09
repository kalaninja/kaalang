use kaalang::kaa;

#[kaa]
fn digital_root(n: u32) -> u32 {
    #[kaa::stage("Reduce the number to one digit.")]
    let (n, finish) = |n| {
        #[kaa::question("Does the number have one digit?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (single, several) = |n| n < 10;

        #[kaa::action("Keep the digit.")]
        let finish = |single, n| n;

        #[kaa::action("Start summing the digits.")]
        let (mut rest, mut sum) = |several, n| (n, 0u32);

        #[kaa::cycle("Add up the digits.")]
        let n = |rest| loop {
            #[kaa::question("Are any digits left?")]
            #[kaa::yes("YES")]
            #[kaa::no("NO")]
            let (more, done) = |rest| rest > 0;

            #[kaa::action("Move the last digit into the sum.")]
            let moved = |more, &mut rest, &mut sum| {
                *sum += *rest % 10;
                *rest /= 10;
            };

            |moved| continue;

            #[kaa::action("Reduce the digit sum next.")]
            let n = |done, sum| sum;
        };
    };

    #[kaa::stage("Return the digit.")]
    |finish| {
        |finish| return finish;
    };
}

/// The inner cycle's output is the stage's own entry, so completing the cycle
/// selects another visit of the same stage.
#[test]
fn a_cycle_output_selects_its_own_stage_again() {
    assert_eq!(digital_root(0), 0);
    assert_eq!(digital_root(7), 7);
    assert_eq!(digital_root(38), 2);
    assert_eq!(digital_root(999_999), 9);
}
