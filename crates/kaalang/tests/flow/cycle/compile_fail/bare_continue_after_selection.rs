use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Repeat without choosing the repeating branch.")]
    loop {
        #[kaa::question("Leave now?")]
        let (_stay, _again) = |flag| flag;

        continue;
    };
}

fn main() {}
