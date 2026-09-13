use kaalang::kaalang;

#[kaalang]
fn end_case_between_breaks(mode: u8) -> u8 {
    loop {
        #[choice("Which route?")]
        #[case("Leave at zero.")]
        #[case("Return seven.")]
        #[case("Leave at another mode.")]
        let (first, finish, last) = |mode| match mode {
            0 => (),
            1 => (),
            _ => (),
        };

        |first| break;

        #[action("Return seven.")]
        let end = |finish| 7;

        |last| break;
    }

    #[action("Return the mode.")]
    let end = |mode| mode;
}

fn main() {}
