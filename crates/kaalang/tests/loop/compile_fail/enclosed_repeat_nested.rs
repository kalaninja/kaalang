use kaalang::kaalang;

#[kaalang]
fn invalid(mut mode: u8) -> u8 {
    'outer: loop {
        loop {
            #[question("Exit immediately?")]
            let (first, check) = |mode| mode == 0;

            |first| break 'outer;

            #[question("Advance once?")]
            let (advance, last) = |check, mode| mode == 1;

            #[action("Advance to the final case.")]
            |advance, &mut mode| *mode = 2;

            |last| break 'outer;
        }
    }

    #[action("Return the selected mode.")]
    let end = |mode| mode;
}

fn main() {}
