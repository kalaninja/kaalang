use kaalang::kaalang;

/// The first selection merges two `String` producers into `text`, a common
/// action moves it into a length, and a second selection follows. Nothing
/// needs to transfer `text` again: its binding is outside the second selection.
#[kaalang]
fn successive_selections_move_a_string(condition: bool, long: bool) -> usize {
    #[question("Which text?")]
    |condition| -> (first, second) { condition };

    #[action("Build the first text.")]
    |first| -> text { String::from("abc") };

    #[action("Build the second text.")]
    |second| -> text { String::from("de") };

    #[action("Measure the merged text.")]
    |text| -> length { text.len() };

    #[question("Is it long enough?")]
    |&length, long| -> (yes, no) { long && *length > 2 };

    #[action("Keep the length.")]
    |yes, length| -> selected { length };

    #[action("Scale the short length.")]
    |no, length| -> selected { length * 100 };

    #[action("Finish with the selected length.")]
    |selected| -> result { selected };
}

#[test]
fn the_text_moves_once_and_the_later_join_leaves_it_behind() {
    assert_eq!(successive_selections_move_a_string(true, true), 3);
    assert_eq!(successive_selections_move_a_string(true, false), 300);
    assert_eq!(successive_selections_move_a_string(false, true), 200);
    assert_eq!(successive_selections_move_a_string(false, false), 200);
}
