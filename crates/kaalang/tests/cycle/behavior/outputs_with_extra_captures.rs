use kaalang::kaalang;

/// Each continuation of a three-output cycle also captures an outer wire, so
/// every exit keeps its own hand-over label. Collapsed, the side exits share
/// one row and each label stands where its route turns down.
#[kaalang]
fn outputs_with_extra_captures(mut state: u8, bonus: u8) -> u8 {
    #[cycle("Settle on a colour.")]
    let (red, green, blue) = loop {
        #[choice("Which colour is it?")]
        #[case("Red.")]
        #[case("Green.")]
        #[case("Blue.")]
        #[case("Not settled yet.")]
        let (red, green, blue, other) = |&state| match *state {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[action("Try again.")]
        let stepped = |other, &mut state| *state -= 1;

        |stepped| continue;
    };

    #[action("Score red.")]
    let score = |red, bonus| 10 + bonus;

    #[action("Score green.")]
    let score = |green, bonus| 20 + bonus;

    #[action("Score blue.")]
    let score = |blue, bonus| 30 + bonus;

    |score| return score;
}

#[test]
fn each_output_adds_the_bonus() {
    assert_eq!(outputs_with_extra_captures(0, 1), 11);
    assert_eq!(outputs_with_extra_captures(1, 2), 22);
    assert_eq!(outputs_with_extra_captures(4, 3), 33);
}
