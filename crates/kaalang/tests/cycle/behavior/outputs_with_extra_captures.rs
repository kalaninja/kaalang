use kaalang::kaa;

/// Each continuation of a three-output cycle also captures an outer wire, so
/// every exit keeps its own hand-over label. Collapsed, the side exits share
/// one row and each label stands where its route turns down.
#[kaa]
fn outputs_with_extra_captures(mut state: u8, bonus: u8) -> u8 {
    #[kaa::cycle("Settle on a colour.")]
    let (red, green, blue) = loop {
        #[kaa::choice("Which colour is it?")]
        #[kaa::case("Red.")]
        #[kaa::case("Green.")]
        #[kaa::case("Blue.")]
        #[kaa::case("Not settled yet.")]
        let (red, green, blue, other) = |&state| match *state {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[kaa::action("Try again.")]
        let stepped = |other, &mut state| *state -= 1;

        |stepped| continue;
    };

    #[kaa::action("Score red.")]
    let score = |red, bonus| 10 + bonus;

    #[kaa::action("Score green.")]
    let score = |green, bonus| 20 + bonus;

    #[kaa::action("Score blue.")]
    let score = |blue, bonus| 30 + bonus;

    |score| return score;
}

#[test]
fn each_output_adds_the_bonus() {
    assert_eq!(outputs_with_extra_captures(0, 1), 11);
    assert_eq!(outputs_with_extra_captures(1, 2), 22);
    assert_eq!(outputs_with_extra_captures(4, 3), 33);
}
