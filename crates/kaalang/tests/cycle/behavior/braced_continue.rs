use kaalang::kaalang;

#[rustfmt::skip] // Preserve the braced continue without an inner semicolon.
#[kaalang]
fn braced_continue(mut remaining: usize) -> usize {
    #[cycle("Count down.")]
    let done = loop {
        #[question("Is there another step?")]
        let (again, done) = |&remaining| *remaining > 0;

        #[action("Count down one step.")]
        |again, &mut remaining| *remaining -= 1;

        |again| { continue };
    };

    |done, remaining| return remaining;
}

#[test]
fn a_braced_continue_without_an_inner_semicolon_repeats() {
    assert_eq!(braced_continue(3), 0);
}
