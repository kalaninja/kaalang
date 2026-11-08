use kaalang::kaa;

#[kaa]
fn invalid(mode: u8) -> u8 {
    #[kaa::choice("Which source?")]
    #[kaa::case("First.")]
    #[kaa::case("Second.")]
    #[kaa::case("Third.")]
    #[kaa::case("Fourth.")]
    let (a, b, c, d) = |mode| {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[kaa::action("First value.")]
    let early = |a| { 1 };

    #[kaa::action("Second value.")]
    let early = |b| { 2 };

    #[kaa::action("Third value.")]
    let late = |c| { 3 };

    #[kaa::action("Fourth value.")]
    let late = |d| { 4 };

    #[kaa::action("Common effect.")]
    || {};

    #[kaa::action("Use the early value.")]
    let result = |early| early;

    #[kaa::action("Use the late value.")]
    let result = |late| late;

    |result| return result;
}

fn main() {}
