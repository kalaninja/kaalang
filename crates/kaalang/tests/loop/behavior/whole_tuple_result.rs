use kaalang::kaalang;

#[kaalang]
fn whole_tuple_result(value: usize) -> (usize, usize) {
    #[cycle("Keep a tuple in one result wire.")]
    let pair = |value| {
        #[action("Build the tuple.")]
        let pair = |value| (value, value + 1);

        |pair| break pair;
    };

    |pair| return pair;
}

#[test]
fn a_tuple_can_remain_one_wire_through_a_cycle_and_return() {
    assert_eq!(whole_tuple_result(4), (4, 5));
}
