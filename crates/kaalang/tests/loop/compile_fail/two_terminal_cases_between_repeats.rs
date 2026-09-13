use kaalang::kaalang;

#[kaalang]
fn two_terminal_cases_between_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance from zero.")]
        #[case("Finish with seven.")]
        #[case("Finish with nine.")]
        #[case("Advance from another mode.")]
        let (left, seven, nine, right) = |mode| match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[action("Finish the flow with seven.")]
        let end = |seven| 7;

        #[action("Finish the flow with nine.")]
        let end = |nine| 9;

        #[action("Set the mode to one.")]
        |left, &mut mode| *mode = 1;

        #[action("Set the mode to one.")]
        |right, &mut mode| *mode = 1;
    }
}

fn main() {}
