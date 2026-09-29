use kaalang::kaalang;

#[kaalang]
fn spin(go: ()) -> ! {
    #[stage("Visit the first stage.")]
    let next = |go| {
        #[action("Move to the second stage.")]
        let next = |go| {};
    };

    #[stage("Visit the second stage.")]
    let go = |next| {
        #[action("Move back to the first stage.")]
        let go = |next| {};
    };
}

#[test]
fn transitions_can_diverge_without_a_terminal_stage() {
    let _: fn(()) -> ! = spin;
}
