use kaalang::kaalang;

#[kaalang]
fn digital_root(n: u32) -> u32 {
    #[stage("Reduce the number to one digit.")]
    let (n, finish) = |n| {
        #[question("Does the number have one digit?")]
        #[yes("YES")]
        #[no("NO")]
        let (single, several) = |n| n < 10;

        #[action("Keep the digit.")]
        let finish = |single, n| n;

        #[action("Start summing the digits.")]
        let (mut rest, mut sum) = |several, n| (n, 0u32);

        #[cycle("Add up the digits.")]
        let n = |rest| loop {
            #[question("Are any digits left?")]
            #[yes("YES")]
            #[no("NO")]
            let (more, done) = |rest| rest > 0;

            #[action("Move the last digit into the sum.")]
            let moved = |more, &mut rest, &mut sum| {
                *sum += *rest % 10;
                *rest /= 10;
            };

            |moved| continue;

            #[action("Reduce the digit sum next.")]
            let n = |done, sum| sum;
        };
    };

    #[stage("Return the digit.")]
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
