use kaalang::kaalang;

/// The `Copy` variant: the merged seed remains in scope and is captured again
/// after the second selection's branches join.
#[kaalang]
fn successive_selections_keep_a_copy(condition: bool, long: bool) -> u32 {
    #[question("Which seed?")]
    let (first, second) = |condition| condition;

    #[action("Build the first seed.")]
    let seed = |first| 3u32;

    #[action("Build the second seed.")]
    let seed = |second| 2u32;

    #[action("Scale the merged seed.")]
    let scaled = |seed| seed * 10;

    #[question("Is it large?")]
    let (yes, no) = |&scaled, long| long && *scaled > 20;

    #[action("Include the original seed.")]
    let include_seed = |yes| true;

    #[action("Use the scaled value alone.")]
    let include_seed = |no| false;

    #[action("Finish with the selected adjustment.")]
    let end = |include_seed, scaled, seed| {
        if include_seed { scaled + seed } else { scaled }
    };
}

#[test]
fn a_copy_wire_stays_available_past_the_second_selection() {
    assert_eq!(successive_selections_keep_a_copy(true, true), 33);
    assert_eq!(successive_selections_keep_a_copy(true, false), 30);
    assert_eq!(successive_selections_keep_a_copy(false, true), 20);
    assert_eq!(successive_selections_keep_a_copy(false, false), 20);
}
