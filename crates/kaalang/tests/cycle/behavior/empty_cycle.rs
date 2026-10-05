use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_cycle() -> usize {
    #[cycle("Repeat forever.")]
    loop {
        continue;
    }
}
