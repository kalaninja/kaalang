use kaalang::kaa;

// Rust lets a block-like statement and the last statement of a block drop the
// semicolon. kaalang gives the semicolon no meaning, so both spellings parse.
#[kaa]
fn statements_without_semicolons() -> String {
    #[kaa::action("Start with an empty text.")]
    let mut text = || String::new();

    #[kaa::cycle("Knock three times.")]
    {
        for _ in 0..3 {
            #[kaa::action("Knock once.")]
            |&mut text| text.push_str("knock ");
        }
    }

    |text| return text
}

#[test]
fn block_statements_parse_without_their_semicolons() {
    assert_eq!(statements_without_semicolons(), "knock knock knock ");
}
