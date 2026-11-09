use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> bool {
    #[kaa::cycle("Try to leave through a break.")]
    let result = |flag| loop {
        |flag| break flag;
    };

    |result| return result;
}

fn main() {}
