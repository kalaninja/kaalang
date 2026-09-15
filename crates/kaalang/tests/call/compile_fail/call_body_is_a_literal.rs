use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[call("Produce one.")]
    let end = |value| 1;

    |end| return end;
}

fn main() {}
