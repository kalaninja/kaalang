use kaalang::kaalang;

#[kaalang]
fn invalid(mode: u8) -> u8 {
    #[cycle("Put a break between repeating routes.")]
    let result = |mut mode| {
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
        |first, &mut mode| *mode = 1;

        |leave, mode| break mode;

        #[action("Set the mode to one.")]
        |last, &mut mode| *mode = 1;
    };

    |result| return result;
}

fn main() {}
