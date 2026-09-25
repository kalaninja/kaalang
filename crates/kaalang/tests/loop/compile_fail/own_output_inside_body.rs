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
        let next = |value| value + 1;

        |next| break next;
    };

    |value| return value;
}

fn main() {}
