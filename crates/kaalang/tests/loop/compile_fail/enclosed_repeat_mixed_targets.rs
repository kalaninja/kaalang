use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    'outer: loop {
        loop {
            #[choice("Leave or advance?")]
            #[case("Leave both loops immediately.")]
            #[case("Advance once.")]
            #[case("Leave both loops after advancing.")]
            #[case("Leave only the inner loop.")]
            let (first, advance, last, inner) = |mode| match mode {
                0 => (),
                1 => (),
                2 => (),
                _ => (),
            };

            |first| break 'outer;

            #[action("Advance to the final case.")]
            |advance, &mut mode| *mode = 2;

            |last| break 'outer;

            |inner| break;
        }

        break;
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

fn main() {}
