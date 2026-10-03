use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    #[cycle("Put an exit between repeating routes.")]
    let leave = loop {
        #[choice("Exit or advance?")]
        #[case("Advance from zero.")]
        #[case("Leave the cycle.")]
        #[case("Advance from another mode.")]
        let (first, leave, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Set the mode to one.")]
        let advanced = |first, &mut mode| *mode = 1;

        #[action("Set the mode to one.")]
        let advanced = |last, &mut mode| *mode = 1;

        |advanced| continue;
    };

    |leave, mode| return mode;
}

fn main() {}
