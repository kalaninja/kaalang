use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_trailing_loop(flag: bool) -> usize {
    #[cycle("Repeat the outer pass forever.")]
    |flag| {
        #[cycle("Repeat the inner check until it leaves.")]
        |flag| {
            #[question("Repeat the inner iteration?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |flag| flag;

            |leave_1| break;

            |iterate_1| continue;
        };

        continue;
    };
}
