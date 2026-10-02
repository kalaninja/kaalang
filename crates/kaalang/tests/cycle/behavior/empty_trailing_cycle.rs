use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn empty_trailing_cycle(flag: bool) -> usize {
    #[cycle("Repeat the outer pass forever.")]
    |flag| {
        #[cycle("Repeat the inner check until it leaves.")]
        let leave_1 = |flag| {
            #[question("Repeat the inner iteration?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_1, leave_1) = |flag| flag;

            |iterate_1| continue;
        };

        |leave_1| continue;
    };
}
