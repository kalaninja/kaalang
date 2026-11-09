use kaalang::kaa;

/// Two exits of different Rust types. Only the selected one
/// exists after the cycle; the outer continuations merge before the return.
#[kaa]
fn alternative_outputs(mut items: Vec<String>) -> Option<String> {
    #[kaa::cycle("Find a nonempty item.")]
    let (found, exhausted) = loop {
        #[kaa::choice("Is another item available?")]
        #[kaa::case("Inspect the item.")]
        #[kaa::case("The collection is exhausted.")]
        let (item, exhausted) = |&mut items| match items.pop() {
            Some(item) => item,
            None => (),
        };

        #[kaa::question("Is the item empty?")]
        let (skip, keep) = |&item| item.is_empty();

        #[kaa::action("Provide the nonempty item.")]
        let found = |keep, item| item;

        |skip| continue;
    };

    #[kaa::action("Return the found item.")]
    let result = |found| Some(found);

    #[kaa::action("Report exhaustion.")]
    let result = |exhausted| None;

    |result| return result;
}

#[test]
fn only_the_selected_output_continues() {
    let items = |values: &[&str]| values.iter().map(|&value| value.to_owned()).collect();
    assert_eq!(
        alternative_outputs(items(&["a", "", ""])),
        Some("a".to_owned())
    );
    assert_eq!(alternative_outputs(items(&["", "b"])), Some("b".to_owned()));
    assert_eq!(alternative_outputs(items(&["", ""])), None);
    assert_eq!(alternative_outputs(Vec::new()), None);
}
