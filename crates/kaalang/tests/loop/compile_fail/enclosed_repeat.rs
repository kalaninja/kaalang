use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    loop {
        #[choice("Exit or advance?")]
        #[case("Exit immediately.")]
        #[case("Advance once.")]
        #[case("Exit after advancing.")]
        let (first, advance, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        |first| break;

        #[action("Advance to the final case.")]
        |advance, &mut mode| *mode = 2;

        |last| break;
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

fn main() {}
