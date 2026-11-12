use kaalang::kaa;

#[kaa]
fn invalid((left, right): (u32, u32)) -> u32 {
    #[kaa::action("Add both halves.")]
    let end = |left, right| { left + right };

    |end| return end;
}

fn main() {}
