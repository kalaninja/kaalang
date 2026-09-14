use kaalang::kaalang;

#[kaalang]
fn invalid(text: String) -> usize {
    #[cycle("Move the value into the cycle.")]
    |text| {
        break;
    };

    #[action("Use the moved value.")]
    let result = |text| text.len();

    |result| return result;
}

fn main() {}
