use kaalang::kaalang;

#[kaalang]
fn result_shapes(value: usize) -> (usize, usize, usize, usize) {
    #[cycle("Produce a named unit result.")]
    let ready = || {
        #[action("Get ready.")]
        let ready = || {};
    };

    #[cycle("Produce a mutable scalar result.")]
    let mut adjusted = |value| {
        #[action("Copy the value.")]
        let adjusted = |value| value;
    };

    #[action("Adjust the mutable cycle result.")]
    |ready, &mut adjusted| *adjusted += 1;

    #[cycle("Declare a singleton output, which does not destructure.")]
    let (single,) = |value| {
        #[action("Copy the value again.")]
        let single = |value| value;
    };

    #[cycle("Hand over a tuple-valued output whole.")]
    let pair = |single| {
        #[action("Build two results.")]
        let pair = |single| (single, single + 1);
    };

    #[action("Destructure the pair after the cycle.")]
    let (left, right) = |pair| pair;

    |adjusted, single, left, right| return (adjusted, single, left, right);
}

#[test]
fn cycle_outputs_hand_over_whole_values() {
    assert_eq!(result_shapes(4), (5, 4, 4, 5));
}
