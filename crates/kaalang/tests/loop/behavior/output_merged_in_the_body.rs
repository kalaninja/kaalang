use kaalang::kaalang;

/// The declared output `taken` is produced on two branches of the body. The two
/// producers merge by the ordinary rules, and the merged wire leaves the cycle.
#[kaalang]
fn output_merged_in_the_body(
    mut items: Vec<String>,
    mut log: Vec<String>,
) -> (Option<String>, Vec<String>) {
    #[cycle("Take the next item.")]
    let taken = {
        #[action("Take an item.")]
        let item = |&mut items| items.pop();

        #[question("Is it worth logging?")]
        let (loud, quiet) = |&item| item.is_some();

        #[action("Log it and keep it.")]
        let taken = |loud, item, &mut log| {
            log.push(format!("{item:?}"));
            item
        };

        #[action("Keep it quietly.")]
        let taken = |quiet, item| item;
    };

    |taken, log| return (taken, log);
}

#[test]
fn both_branches_hand_over_the_merged_output() {
    let (taken, log) = output_merged_in_the_body(vec!["a".to_owned()], Vec::new());
    assert_eq!(taken, Some("a".to_owned()));
    assert_eq!(log, ["Some(\"a\")"]);
    let (taken, log) = output_merged_in_the_body(Vec::new(), Vec::new());
    assert_eq!(taken, None);
    assert!(log.is_empty());
}
