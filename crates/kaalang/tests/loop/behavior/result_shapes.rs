use kaalang::kaalang;

#[kaalang]
fn result_shapes(value: usize) -> (usize, usize, usize, usize) {
    #[cycle("Complete without outputs.")]
    let () = || {
        break;
    };

    #[cycle("Produce a named unit result.")]
    let ready = || {
        || break ();
    };

    #[cycle("Produce a mutable scalar result.")]
    let mut adjusted = |value| {
        |value| break value;
    };

    #[action("Adjust the mutable cycle result.")]
    |ready, &mut adjusted| *adjusted += 1;

    #[cycle("Destructure a singleton tuple result.")]
    let (single,) = |value| {
        #[action("Build the singleton tuple.")]
        let tuple = |value| (value,);

        |tuple| break tuple;
    };

    #[cycle("Destructure two result wires.")]
    let (left, right) = |single| {
        #[action("Build two results.")]
        let pair = |single| (single, single + 1);

        |pair| break pair;
    };

    |adjusted, single, left, right| return (adjusted, single, left, right);
}

#[test]
fn cycle_results_follow_the_authored_output_pattern() {
    assert_eq!(result_shapes(4), (5, 4, 4, 5));
}
