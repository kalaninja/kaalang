use kaalang::kaalang;

#[kaalang]
fn for_ignored_item(times: usize) -> String {
    #[action("Start with an empty text.")]
    let mut text = || String::new();

    #[cycle("Knock as many times as asked.")]
    |times| {
        for _ in 0..times {
            #[action("Knock once.")]
            |&mut text| text.push_str("knock ");
        }
    };

    |text| return text;
}

#[test]
fn an_ignored_item_still_repeats_the_body() {
    assert_eq!(for_ignored_item(0), "");
    assert_eq!(for_ignored_item(2), "knock knock ");
}
