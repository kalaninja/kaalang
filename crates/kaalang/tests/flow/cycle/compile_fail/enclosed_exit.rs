use kaalang::kaa;

#[kaa]
fn invalid(mut mode: u8) -> u8 {
    #[kaa::cycle("Put an exit between repeating routes.")]
    let leave = loop {
        #[kaa::choice("Exit or advance?")]
        #[kaa::case("Advance from zero.")]
        #[kaa::case("Leave the cycle.")]
        #[kaa::case("Advance from another mode.")]
        let (first, leave, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        #[kaa::action("Set the mode to one.")]
        let advanced = |first, &mut mode| *mode = 1;

        #[kaa::action("Set the mode to one.")]
        let advanced = |last, &mut mode| *mode = 1;

        |advanced| continue;
    };

    |leave, mode| return mode;
}

fn main() {}
