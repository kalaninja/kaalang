use kaalang::kaa;

/// The first selection merges two `String` producers into `text`, a common
/// action moves it into a length, and a second selection follows. Nothing
/// needs to transfer `text` again: its binding is outside the second selection.
#[kaa]
fn successive_selections_move_a_string(condition: bool, long: bool) -> usize {
    #[kaa::question("Which text?")]
    let (first, second) = |condition| condition;

    #[kaa::action("Build the first text.")]
    let text = |first| String::from("abc");

    #[kaa::action("Build the second text.")]
    let text = |second| String::from("de");

    #[kaa::action("Measure the merged text.")]
    let length = |text| text.len();

    #[kaa::question("Is it long enough?")]
    let (yes, no) = |&length, long| long && *length > 2;

    #[kaa::action("Keep the length.")]
    let selected = |yes, length| length;

    #[kaa::action("Scale the short length.")]
    let selected = |no, length| length * 100;

    #[kaa::action("Finish with the selected length.")]
    let end = |selected| selected;

    |end| return end;
}

#[test]
fn the_text_moves_once_and_the_later_join_leaves_it_behind() {
    assert_eq!(successive_selections_move_a_string(true, true), 3);
    assert_eq!(successive_selections_move_a_string(true, false), 300);
    assert_eq!(successive_selections_move_a_string(false, true), 200);
    assert_eq!(successive_selections_move_a_string(false, false), 200);
}
