use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Check the flag.")]
    let leave = |flag| loop {
        #[kaa::question("Repeat?")]
        let (_again, leave) = |flag| {
            if flag {
                break;
            }
            false
        };
    };

    |leave| return;
}

fn main() {}
