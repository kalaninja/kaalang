use kaalang::kaalang;

#[kaalang]
fn two_terminal_cases_between_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Finish with seven.")]
        #[case("Finish with nine.")]
        #[case("Advance on the right.")]
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

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    }
}

fn main() {}
