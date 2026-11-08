use kaalang::kaa;

#[allow(dead_code)]
#[kaa]
fn empty_cycle() -> usize {
    #[kaa::cycle("Repeat forever.")]
    loop {
        continue;
    }
}
