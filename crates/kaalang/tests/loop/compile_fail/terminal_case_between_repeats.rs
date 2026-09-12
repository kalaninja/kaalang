use kaalang::kaalang;

#[kaalang]
fn terminal_case_between_repeats(mut mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Finish in the middle.")]
        #[case("Advance on the right.")]
        let (left, middle, right) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Finish from the middle case.")]
        let end = |middle| 7;

        #[action("Advance through the left case.")]
        |left, &mut mode| *mode = 1;

        #[action("Advance through the right case.")]
        |right, &mut mode| *mode = 1;
    }
}

fn main() {}
