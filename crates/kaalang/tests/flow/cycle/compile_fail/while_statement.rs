use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> ! {
    #[kaa::question("Repeat?")]
    while (|flag| flag) {}
}

fn main() {}
