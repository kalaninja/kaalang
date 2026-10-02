use kaalang::kaalang;

#[kaalang]
fn braced_continue_with_semicolon(mut remaining: usize) -> usize {
    #[cycle("Count down.")]
    let done = {
        #[question("Is there another step?")]
        let (again, done) = |&remaining| *remaining > 0;

        #[action("Count down one step.")]
        |again, &mut remaining| *remaining -= 1;

        |again| {
            continue;
        };
    };

    |done, remaining| return remaining;
}

#[test]
fn a_braced_continue_with_an_inner_semicolon_repeats() {
    assert_eq!(braced_continue_with_semicolon(3), 0);
}
