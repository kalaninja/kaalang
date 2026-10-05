use kaalang::kaalang;

#[kaalang]
fn empty_output_pattern() -> ! {
    #[cycle("Repeat with an explicit empty output pattern.")]
    let () = loop {
        continue;
    };
}

#[test]
fn an_empty_output_pattern_declares_no_cycle_outputs() {
    let _: fn() -> ! = empty_output_pattern;
}
