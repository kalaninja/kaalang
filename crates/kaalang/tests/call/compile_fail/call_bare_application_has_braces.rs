use kaalang::kaalang;

fn record() {}

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[call]
    {
        record();
    };

    #[action("Keep the value.")]
    let end = |value| value;

    |end| return end;
}

fn main() {}
