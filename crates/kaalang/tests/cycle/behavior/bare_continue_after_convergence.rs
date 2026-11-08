use kaalang::kaa;

#[kaa]
fn bare_continue_after_convergence(flag: bool) -> ! {
    #[kaa::cycle("Repeat after both branches converge.")]
    loop {
        #[kaa::question("Take the first route?")]
        let (first, second) = |flag| flag;

        #[kaa::action("Finish the first route.")]
        let ready = |first| {};

        #[kaa::action("Finish the second route.")]
        let ready = |second| {};

        #[kaa::action("Use the merged wire.")]
        |ready| {};

        continue;
    }
}

#[test]
fn a_bare_continue_can_follow_a_closed_selection() {
    let _: fn(bool) -> ! = bare_continue_after_convergence;
}
