use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[question("Does the predecessor stay positive?")]
    |&value| -> (yes, no) { value.checked_sub(1)? > 0 };

    #[action("Produce the yes result.")]
    |yes, value| -> result { value };

    #[action("Produce the no result.")]
    |no, value| -> result { value + 1 };

    #[end]
    |result| {};
}

fn main() {}
