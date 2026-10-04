use kaalang::kaalang;

#[kaalang]
fn for_internal_names(
    __kaalang_unnamed_done_0: u32,
    __kaalang_unnamed_item_0: u32,
    __kaalang_items: u32,
) -> u32 {
    #[cycle("Complete one iteration without naming its item or output.")]
    for _ in 0..1 {}

    #[action("Add the authored values.")]
    let __kaalang_unnamed_done_0_ =
        |__kaalang_unnamed_done_0, __kaalang_unnamed_item_0, __kaalang_items| {
            __kaalang_unnamed_done_0 + __kaalang_unnamed_item_0 + __kaalang_items
        };

    |__kaalang_unnamed_done_0_| return __kaalang_unnamed_done_0_;
}

#[test]
fn generated_wires_avoid_parameters_and_later_outputs() {
    assert_eq!(for_internal_names(1, 2, 3), 6);
}
