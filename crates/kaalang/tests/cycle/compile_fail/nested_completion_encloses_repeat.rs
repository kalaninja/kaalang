use kaalang::kaa;

#[kaa]
fn invalid(mut mode: u8) -> u8 {
    #[kaa::cycle("Run the outer cycle.")]
    let selected = |mode| loop {
        #[kaa::cycle("Put a repeating route between completions.")]
        let selected = loop {
            #[kaa::choice("Leave or advance?")]
            #[kaa::case("Leave immediately.")]
            #[kaa::case("Advance once.")]
            #[kaa::case("Leave after advancing.")]
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
    };

    |selected| return selected;
}

fn main() {}
