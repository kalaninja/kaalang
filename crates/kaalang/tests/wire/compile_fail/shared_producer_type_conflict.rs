use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which width?")]
    |condition| -> (narrow, wide) { condition };

    #[action("Produce one shared default.")]
    || -> shared { Default::default() };

    #[action("Use it as a byte.")]
    |narrow, shared| -> result {
        let byte: u8 = shared;
        u32::from(byte)
    };

    #[action("Use it as a word.")]
    |wide, shared| -> result {
        let word: u16 = shared;
        u32::from(word)
    };

    #[end]
    |result| {};
}

fn main() {}
