use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    #[question("Repeat?")]
    while (|flag| flag) {}
}

fn main() {}
