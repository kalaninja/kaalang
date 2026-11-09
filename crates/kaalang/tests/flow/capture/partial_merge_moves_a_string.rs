use std::cell::Cell;

use kaalang::kaa;

/// A non-`Copy` value passes the inner join of a partial merge and then the
/// outer one, moving at each step.
#[kaa]
fn partial_merge_moves_a_string(source: u8, order: &Cell<u32>) -> String {
    #[kaa::choice("Which source?")]
    #[kaa::case("First source.")]
    #[kaa::case("Second source.")]
    #[kaa::case("Third source.")]
    let (first, second, third) = |source| match source {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Build the text from the first source.")]
    let partial = |first, &order| {
        order.set(order.get() * 10 + 1);
        String::from("first")
    };

    #[kaa::action("Build the text from the second source.")]
    let partial = |second, &order| {
        order.set(order.get() * 10 + 2);
        String::from("second")
    };

    #[kaa::action("Extend the partially merged text.")]
    let shared = |partial, &order| {
        order.set(order.get() * 10 + 3);
        partial + "-extended"
    };

    #[kaa::action("Build the text from the third source.")]
    let shared = |third, &order| {
        order.set(order.get() * 10 + 4);
        String::from("third")
    };

    #[kaa::action("Finish with the merged text.")]
    let end = |shared, &order| {
        order.set(order.get() * 10 + 5);
        shared
    };

    |end| return end;
}

#[test]
fn each_case_moves_its_text_through_the_joins_it_reaches() {
    for (source, expected, expected_order) in [
        (0, "first-extended", 135),
        (1, "second-extended", 235),
        (2, "third", 45),
    ] {
        let order = Cell::new(0);
        assert_eq!(partial_merge_moves_a_string(source, &order), expected);
        assert_eq!(order.get(), expected_order);
    }
}
