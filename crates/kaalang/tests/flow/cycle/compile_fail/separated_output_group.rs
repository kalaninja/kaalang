use kaalang::kaa;

#[kaa]
fn invalid(mut state: u8) -> u8 {
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
    let result = |green| 20;

    #[kaa::action("Score blue.")]
    let score = |blue| 30;

    #[kaa::action("Keep the score.")]
    let result = |score| score;

    |result| return result;
}

fn main() {}
