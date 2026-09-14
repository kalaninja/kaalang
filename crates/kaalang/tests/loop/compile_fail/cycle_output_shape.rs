use kaalang::kaalang;

#[kaalang]
fn invalid(value: usize) -> (usize, usize) {
    #[cycle("Destructure two outputs from one value.")]
    let (left, right) = |value| {
        |value| break value;
    };

    |left, right| return (left, right);
}

fn main() {}
