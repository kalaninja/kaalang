use kaalang::kaalang;

/// Owned alternative outputs of an unconstrained generic type in a `const fn`.
/// Each output moves the same value into its own result block.
#[kaalang]
const fn generic_owned_outputs<T>(value: T, mut remaining: usize, take_left: bool) -> T {
    #[cycle("Count down, then pick a side.")]
    let (left, right) = {
        #[question("Is any count left?")]
        let (again, decide) = |&remaining| *remaining > 0;

        #[action("Count down.")]
        let stepped = |again, &mut remaining| *remaining -= 1;

        |stepped| continue;

        #[question("Take the left side?")]
        let (take, skip) = |decide, take_left| take_left;

        #[action("Hand the value to the left.")]
        let left = |take, value| value;

        #[action("Hand the value to the right.")]
        let right = |skip, value| value;
    };

    #[action("Keep the left value.")]
    let chosen = |left| left;

    #[action("Keep the right value.")]
    let chosen = |right| right;

    |chosen| return chosen;
}

#[test]
fn an_owned_value_leaves_through_either_output() {
    const LEFT: u8 = generic_owned_outputs(7, 3, true);
    assert_eq!(LEFT, 7);
    assert_eq!(generic_owned_outputs(String::from("x"), 2, false), "x");
}
