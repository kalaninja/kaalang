use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> u32 {
    #[question("Which route?")]
    let (first, second) = |flag| flag;

    #[action("Produce the first value.")]
    let value = |first| 1;

    #[cycle("Read the value this cycle produces.")]
    let value = |second| {
        #[action("Read the value.")]
        let value = |value| value + 1;
    };

    |value| return value;
}

fn main() {}
