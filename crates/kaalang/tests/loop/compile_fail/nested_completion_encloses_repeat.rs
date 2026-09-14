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

            |first, mode| break mode;

            #[action("Advance to the final case.")]
            |advance, &mut mode| *mode = 2;

            |last, mode| break mode;
        };

        |nested| break nested;
    };

    |result| return result;
}

fn main() {}
