use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[action("Produce one shared default.")]
    let shared = || { Default::default() };

    #[question("Which width?")]
    let (narrow, wide) = |condition| { condition };

    #[action("Use it as a byte.")]
    let end = |narrow, shared| {
        let byte: u8 = shared;
        u32::from(byte)
    };

    #[action("Use it as a word.")]
    let end = |wide, shared| {
        let word: u16 = shared;
        u32::from(word)
    };

    |end| return end;
}

fn main() {}
