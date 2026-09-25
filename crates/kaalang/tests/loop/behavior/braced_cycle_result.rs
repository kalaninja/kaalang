use kaalang::kaalang;

#[kaalang]
fn braced_cycle_result(mut count: usize) -> usize {
    #[cycle("Count down to zero.")]
    let zero = {
        #[question("Is the count zero?")]
        let (done, again) = |count| count == 0;

        |done, count| break count;

        #[action("Count down.")]
        |again, &mut count| *count -= 1;
    };

    |zero| return zero;
}

#[test]
fn a_braced_cycle_without_a_gate_hands_over_its_result() {
    for count in [0, 1, 5] {
        assert_eq!(braced_cycle_result(count), 0);
    }
}
