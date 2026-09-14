use kaalang::kaalang;

#[kaalang]
fn mutate_in_selections(mut text: String, count: u8, mode: u8) -> String {
    #[question("Use the choice after changing the text?")]
    let (yes, no) = |&mut text, mut count| {
        text.push('q');
        count += 1;
        count == 1
    };

    #[choice("Change the text and choose a suffix.")]
    #[case("First suffix.")]
    #[case("Second suffix.")]
    let (first, second) = |yes, &mut text, mut mode| match {
        text.push('c');
        mode += 1;
        mode
    } {
        1 => String::from("a"),
        _ => String::from("b"),
    };

    #[action("Consume and extend the first case value.")]
    let selected = |mut first, &mut text| {
        first.push('1');
        text.push('x');
        first
    };

    #[action("Consume and extend the second case value.")]
    let selected = |mut second, &mut text| {
        second.push('2');
        text.push('y');
        second
    };

    #[action("Skip the choice.")]
    let selected = |no| String::from("skip");

    #[action("Return the text and selected suffix.")]
    let end = |text, selected| text + &selected;

    |end| return end;
}

#[test]
fn selections_mutate_once_and_only_selected_branches_run() {
    for (count, mode, expected) in [(0, 0, "qcxa1"), (0, 1, "qcyb2"), (1, 0, "qskip")] {
        assert_eq!(mutate_in_selections(String::new(), count, mode), expected);
    }
}
