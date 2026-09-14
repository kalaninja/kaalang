use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_loop() -> usize {
    #[cycle("Repeat forever.")]
    || {};
}
