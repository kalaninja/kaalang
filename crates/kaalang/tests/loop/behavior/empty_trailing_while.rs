use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_trailing_while(flag: bool) -> usize {
    loop {
        #[question("Repeat the inner iteration?")]
        while (|&flag| *flag) {}
    }
}
