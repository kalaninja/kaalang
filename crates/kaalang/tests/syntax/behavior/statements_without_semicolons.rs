use kaalang::kaalang;

// Rust lets a block-like statement and the last statement of a block drop the
// semicolon. kaalang gives the semicolon no meaning, so both spellings parse.
#[kaalang]
fn statements_without_semicolons() -> String {
    #[action("Start with an empty text.")]
    let mut text = || String::new();

    #[cycle("Knock three times.")]
    {
        for _ in 0..3 {
            #[action("Knock once.")]
            |&mut text| text.push_str("knock ");
        }
    }

    |text| return text
}

#[test]
fn block_statements_parse_without_their_semicolons() {
    assert_eq!(statements_without_semicolons(), "knock knock knock ");
}
