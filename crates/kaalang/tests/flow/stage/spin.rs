use kaalang::kaa;

#[kaa]
fn spin(go: ()) -> ! {
    #[kaa::stage("Visit the first stage.")]
    let next = |go| {
        #[kaa::action("Move to the second stage.")]
        let next = |go| {};
    };

    #[kaa::stage("Visit the second stage.")]
    let go = |next| {
        #[kaa::action("Move back to the first stage.")]
        let go = |next| {};
    };
}

#[test]
fn transitions_can_diverge_without_a_terminal_stage() {
    let _: fn(()) -> ! = spin;
}
