use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_trailing_loop(flag: bool) -> usize {
    loop {
        |&flag| loop {
            #[question("Repeat the inner iteration?")]
            #[yes("YES")]
            #[no("NO")]
            let (_iterate_1, leave_1) = |&flag| *flag;

            |leave_1| break;
        };
    }
}
