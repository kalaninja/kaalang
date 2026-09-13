use kaalang::kaalang;

#[kaalang]
fn terminal_case_between_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance from zero.")]
        #[case("Return seven.")]
        #[case("Advance from another mode.")]
        let (left, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Return seven.")]
        let end = |middle| 7;

        #[action("Set the mode to one.")]
        |left, &mut mode| *mode = 1;

        #[action("Set the mode to one.")]
        |right, &mut mode| *mode = 1;
    }
}

fn main() {}
