use kaalang::kaa;

fn log() {}

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::action("Log the arrival.")]
    log();

    #[kaa::action("Keep the value.")]
    let end = |value| value;

    |end| return end;
}

fn main() {}
