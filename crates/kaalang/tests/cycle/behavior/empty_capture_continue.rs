use kaalang::kaalang;

#[kaalang]
fn empty_capture_continue() -> ! {
    #[cycle("Repeat without an output.")]
    {
        || continue;
    };
}

#[test]
fn an_empty_capture_list_can_repeat_a_cycle() {
    let _: fn() -> ! = empty_capture_continue;
}
