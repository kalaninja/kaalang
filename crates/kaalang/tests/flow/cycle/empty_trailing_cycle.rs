use kaalang::kaa;

#[allow(dead_code)]
#[kaa]
fn empty_trailing_cycle(flag: bool) -> usize {
    #[kaa::cycle("Repeat the outer pass forever.")]
    |flag| loop {
        #[kaa::cycle("Repeat the inner check until it leaves.")]
        let leave_1 = |flag| loop {
            #[kaa::question("Repeat the inner iteration?")]
            #[kaa::yes("YES")]
            #[kaa::no("NO")]
            let (iterate_1, leave_1) = |flag| flag;

            |iterate_1| continue;
        };

        |leave_1| continue;
    };
}
