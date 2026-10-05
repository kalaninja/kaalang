use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    #[cycle("Run the outer cycle.")]
    let selected = |mode| loop {
        #[cycle("Put a repeating route between completions.")]
        let selected = loop {
            #[choice("Leave or advance?")]
            #[case("Leave immediately.")]
            #[case("Advance once.")]
            #[case("Leave after advancing.")]
            let (first, advance, last) = |mode| match mode {
                0 => (),
                1 => (),
                _ => (),
            };

            #[action("Keep the immediate result.")]
            let selected = |first, mode| mode;

            #[action("Advance to the final case.")]
            |advance, &mut mode| *mode = 2;

            #[action("Keep the later result.")]
            let selected = |last, mode| mode;

            |advance| continue;
        };
    };

    |selected| return selected;
}

fn main() {}
