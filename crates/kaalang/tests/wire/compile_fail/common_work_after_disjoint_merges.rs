use kaalang::kaalang;

#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    #[case("Fourth.")]
    |mode| -> (a, b, c, d) {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("First value.")]
    |a| -> early { 1u8 };

    #[action("Second value.")]
    |b| -> early { 2u8 };

    #[action("Third value.")]
    |c| -> late { 3u8 };

    #[action("Fourth value.")]
    |d| -> late { 4u8 };

    #[action("Common effect.")]
    || {};

    #[action("Finish the early half.")]
    |early| -> result { early };

    #[action("Finish the late half.")]
    |late| -> result { late };
}

fn main() {}
