use kaalang::kaalang;

#[kaalang]
fn invalid(mut state: u8) -> u8 {
    #[cycle("Settle on a colour.")]
    let (red, green, blue) = {
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
    let result = |green| 20;

    #[action("Score blue.")]
    let score = |blue| 30;

    #[action("Keep the score.")]
    let result = |score| score;

    |result| return result;
}

fn main() {}
