use kaalang::kaalang;

#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[choice("Which source?")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    #[case("Fourth.")]
    let (a, b, c, d) = |mode| {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("First value.")]
    let early = |a| { 1 };

    #[action("Second value.")]
    let early = |b| { 2 };

    #[action("Third value.")]
    let late = |c| { 3 };

    #[action("Fourth value.")]
    let late = |d| { 4 };

    #[action("Common effect.")]
    || {};

    #[action("Finish the early half.")]
    let end = |early| { early };

    #[action("Finish the late half.")]
    let end = |late| { late };
}

fn main() {}
