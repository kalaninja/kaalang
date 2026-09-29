use kaalang::kaalang;

#[kaalang]
fn braced_cycle_result(mut count: usize) -> usize {
    #[cycle("Count down to zero.")]
    let done = {
        #[question("Is the count zero?")]
        let (done, again) = |count| count == 0;

        #[action("Count down.")]
        |again, &mut count| *count -= 1;

        |again| continue;
    };

    |done, count| return count;
}

#[test]
fn a_braced_cycle_without_a_gate_hands_over_its_result() {
    for count in [0, 1, 5] {
        assert_eq!(braced_cycle_result(count), 0);
    }
}
