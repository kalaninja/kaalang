use kaalang::kaa;

#[kaa]
fn invalid() -> u8 {
    #[kaa::action("Produce the result.")]
    #[kaa::yes]
    let end = || { 1 };

    |end| return end;
}

fn main() {}
