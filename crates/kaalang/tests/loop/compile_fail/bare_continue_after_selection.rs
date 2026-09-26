use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Repeat without choosing the repeating branch.")]
    {
        #[question("Leave now?")]
        let (leave, _again) = |flag| flag;

        |leave| break;

        continue;
    };

    return;
}

fn main() {}
