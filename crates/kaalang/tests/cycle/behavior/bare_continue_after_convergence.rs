use kaalang::kaalang;

#[kaalang]
fn bare_continue_after_convergence(flag: bool) -> ! {
    #[cycle("Repeat after both branches converge.")]
    {
        #[question("Take the first route?")]
        let (first, second) = |flag| flag;

        #[action("Finish the first route.")]
        let ready = |first| {};

        #[action("Finish the second route.")]
        let ready = |second| {};

        #[action("Use the merged wire.")]
        |ready| {};

        continue;
    };
}

#[test]
fn a_bare_continue_can_follow_a_closed_selection() {
    let _: fn(bool) -> ! = bare_continue_after_convergence;
}
