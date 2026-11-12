use kaalang::kaa;

/// Three declared outputs leave the cycle in declaration order and continue
/// like the cases of a choice.
#[kaa]
fn three_outputs(mut state: u8) -> u8 {
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
    let score = |red| 10;

    #[kaa::action("Score green.")]
    let score = |green| 20;

    #[kaa::action("Score blue.")]
    let score = |blue| 30;

    |score| return score;
}

#[test]
fn each_output_takes_its_own_continuation() {
    assert_eq!(three_outputs(0), 10);
    assert_eq!(three_outputs(1), 20);
    assert_eq!(three_outputs(2), 30);
    assert_eq!(three_outputs(5), 30);
}
