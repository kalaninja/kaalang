use kaalang::kaalang;

#[kaalang]
fn invalid((left, right): (u32, u32)) -> u32 {
    #[action("Add both halves.")]
    let end = |left, right| { left + right };

    |end| return end;
}

fn main() {}
