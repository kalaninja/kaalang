use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Check the flag.")]
    |flag| {
        #[question("Repeat?")]
        let (_again, leave) = |flag| {
            if flag {
                break;
            }
            false
        };

        |leave| break;
    };

    return;
}

fn main() {}
