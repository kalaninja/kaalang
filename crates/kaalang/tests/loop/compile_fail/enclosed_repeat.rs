use kaalang::kaalang;

#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[cycle("Put a repeating route between breaks.")]
    let result = |mut mode| {
        #[choice("Exit or advance?")]
        #[case("Exit immediately.")]
        #[case("Advance once.")]
        #[case("Exit after advancing.")]
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

    |result| return result;
}

fn main() {}
