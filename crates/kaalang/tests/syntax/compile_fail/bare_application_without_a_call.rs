use kaalang::kaalang;

fn log() {}

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[action("Log the arrival.")]
    log();

    #[action("Keep the value.")]
    let end = |value| value;

    |end| return end;
}

fn main() {}
