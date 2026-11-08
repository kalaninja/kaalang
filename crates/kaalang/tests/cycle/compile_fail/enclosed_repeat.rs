use kaalang::kaa;

#[kaa]
fn invalid(mut mode: u8) -> u8 {
    #[kaa::cycle("Put a repeating route between exits.")]
    let selected = loop {
        #[kaa::choice("Exit or advance?")]
        #[kaa::case("Exit immediately.")]
        #[kaa::case("Advance once.")]
        #[kaa::case("Exit after advancing.")]
        let (first, advance, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[kaa::action("Keep the immediate result.")]
        let selected = |first, mode| mode;

        #[kaa::action("Advance to the final case.")]
        |advance, &mut mode| *mode = 2;

        #[kaa::action("Keep the later result.")]
        let selected = |last, mode| mode;

        |advance| continue;
    };

    |selected| return selected;
}

fn main() {}
