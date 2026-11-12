use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> ! {
    #[kaa::question("Run?")]
    let (_run, _skip) = |flag| flag;

    #[kaa::cycle("Enter without selecting a branch.")]
    loop {
        continue;
    };
}

fn main() {}
