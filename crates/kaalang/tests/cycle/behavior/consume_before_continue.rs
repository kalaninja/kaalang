use kaalang::kaalang;

#[kaalang]
fn consume_before_continue(mut remaining: usize) -> Vec<String> {
    #[action("Initialize the collection.")]
    let mut values = || Vec::new();

    #[cycle("Collect one value per iteration.")]
    let done = {
        #[question("Is another value needed?")]
        let (again, done) = |&remaining| *remaining > 0;

        #[action("Create an owned value.")]
        let value = |again, &mut remaining| {
            *remaining -= 1;
            String::from("x")
        };

        #[action("Consume the owned value.")]
        let kept = |value, &mut values| values.push(value);

        |kept| continue;
    };

    |done, values| return values;
}

#[test]
fn a_unit_wire_keeps_branch_ancestry_after_an_owned_value_moves() {
    assert_eq!(consume_before_continue(3), ["x", "x", "x"]);
}
