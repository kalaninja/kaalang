use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    loop {
        continue;
    }
}

fn main() {}
