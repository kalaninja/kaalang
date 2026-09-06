use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[question("Is the value large?")]
    |&value| -> (large, small) {
        if *value == 0 {
            return 0;
        }
        *value > 10
    };

    #[action("Produce the large result.")]
    |large, value| -> result { value };

    #[action("Produce the small result.")]
    |small, value| -> result { value + 1 };
}

fn main() {}
