use kaalang::kaa;

#[kaa]
fn empty_capture_continue() -> ! {
    #[kaa::cycle("Repeat without an output.")]
    loop {
        || continue;
    }
}

#[test]
fn an_empty_capture_list_can_repeat_a_cycle() {
    let _: fn() -> ! = empty_capture_continue;
}
