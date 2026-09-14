use kaalang::kaalang;

#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[cycle("Run the outer cycle.")]
    let result = |mode| {
        #[cycle("Put a repeating route between completions.")]
        let nested = |mut mode| {
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

        |selected| break selected;
        };

        |nested| break nested;
    };

    |result| return result;
}

fn main() {}
