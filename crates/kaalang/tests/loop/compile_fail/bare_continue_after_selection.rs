use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Repeat without choosing the repeating branch.")]
    {
        #[question("Leave now?")]
        let (_stay, _again) = |flag| flag;

        continue;
    };
}

fn main() {}
