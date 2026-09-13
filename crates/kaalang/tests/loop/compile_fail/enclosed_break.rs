use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    loop {
        #[choice("Exit or advance?")]
        #[case("Advance from zero.")]
        #[case("Leave the loop.")]
        #[case("Advance from another mode.")]
        let (first, leave, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Set the mode to one.")]
        |first, &mut mode| *mode = 1;

        |leave| break;

        #[action("Set the mode to one.")]
        |last, &mut mode| *mode = 1;
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

fn main() {}
