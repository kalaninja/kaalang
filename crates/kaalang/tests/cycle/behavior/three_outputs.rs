use kaalang::kaalang;

/// Three declared outputs leave the cycle in declaration order and continue
/// like the cases of a choice.
#[kaalang]
fn three_outputs(mut state: u8) -> u8 {
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
    let score = |red| 10;

    #[action("Score green.")]
    let score = |green| 20;

    #[action("Score blue.")]
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
