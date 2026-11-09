use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> u32 {
    #[kaa::question("Which route?")]
    let (first, second) = |flag| flag;

    #[kaa::action("Produce the first value.")]
    let value = |first| 1;

    #[kaa::cycle("Read the value this cycle produces.")]
    let value = |second| loop {
        #[kaa::action("Read the value.")]
        let value = |value| value + 1;
    };

    |value| return value;
}

fn main() {}
