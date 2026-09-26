use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Check the flag.")]
    let leave = |flag| {
        #[question("Repeat?")]
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
