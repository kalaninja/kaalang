use kaalang::kaalang;

#[kaalang]
fn invalid((left, right): (u32, u32)) -> u32 {
    #[action("Add both halves.")]
    |left, right| -> sum { left + right };

    #[end]
    |sum| {};
}

fn main() {}
