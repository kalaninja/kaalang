use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    loop {
        #[choice("Exit or advance?")]
        #[case("Advance on the left.")]
        #[case("Exit in the middle.")]
        #[case("Advance on the right.")]
        let (first, leave, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Advance through the left case.")]
        |first, &mut mode| *mode = 1;

        |leave| break;

        #[action("Advance through the right case.")]
        |last, &mut mode| *mode = 1;
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

fn main() {}
