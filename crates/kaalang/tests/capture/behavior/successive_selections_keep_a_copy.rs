use kaalang::kaalang;

/// The `Copy` variant: the merged seed remains in scope and is captured again
/// after the second selection's branches join.
#[kaalang]
fn successive_selections_keep_a_copy(condition: bool, long: bool) -> u32 {
    #[question("Which seed?")]
    |condition| -> (first, second) { condition };

    #[action("Build the first seed.")]
    |first| -> seed { 3u32 };

    #[action("Build the second seed.")]
    |second| -> seed { 2u32 };

    #[action("Scale the merged seed.")]
    |seed| -> scaled { seed * 10 };

    #[question("Is it large?")]
    |&scaled, long| -> (yes, no) { long && *scaled > 20 };

    #[action("Include the original seed.")]
    |yes| -> include_seed { true };

    #[action("Use the scaled value alone.")]
    |no| -> include_seed { false };

    #[action("Finish with the selected adjustment.")]
    |include_seed, scaled, seed| -> result { if include_seed { scaled + seed } else { scaled } };
}

#[test]
fn a_copy_wire_stays_available_past_the_second_selection() {
    assert_eq!(successive_selections_keep_a_copy(true, true), 33);
    assert_eq!(successive_selections_keep_a_copy(true, false), 30);
    assert_eq!(successive_selections_keep_a_copy(false, true), 20);
    assert_eq!(successive_selections_keep_a_copy(false, false), 20);
}
