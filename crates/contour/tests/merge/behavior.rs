use contour::contour;

#[contour]
fn merge_question(condition: bool) -> String {
    #[question("Take the yes branch?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes result.")]
    |yes| -> yes_value { "yes" };

    #[action("Build the no result.")]
    |no| -> no_value { "no" };

    #[merge]
    |yes_value, no_value| -> (selected,) {};

    #[action("Uppercase the selected result.")]
    |selected| -> result { selected.to_uppercase() };
}

#[contour]
fn merge_choice(value: i32) -> String {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (negative, zero, positive) {
        match value {
            ..0 => "negative",
            0 => "zero",
            _ => "positive",
        }
    };

    #[merge]
    |negative, zero, positive| -> selected {};

    #[action("Label the selected result.")]
    |selected| -> result { format!("merged: {selected}") };
}

#[contour]
fn partially_merge_choice(value: i32, suffix: String) -> String {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (negative, zero, positive) {
        match value {
            ..0 => (),
            0 => (),
            _ => (),
        }
    };

    #[action("Build the negative result.")]
    |negative| -> negative_value { "negative" };

    #[action("Build the zero result.")]
    |zero| -> zero_value { "zero" };

    #[action("Return the positive result.")]
    |positive, suffix| -> positive_result { format!("positive{suffix}") };

    #[merge]
    |negative_value, zero_value| -> selected {};

    #[action("Uppercase and suffix the merged result.")]
    |selected, suffix| -> result { format!("{}{suffix}", selected.to_uppercase()) };
}

#[test]
fn question_merge_transforms_the_selected_value() {
    assert_eq!(merge_question(true), "YES");
    assert_eq!(merge_question(false), "NO");
}

#[test]
fn choice_merge_transforms_every_selected_value() {
    assert_eq!(merge_choice(-1), "merged: negative");
    assert_eq!(merge_choice(0), "merged: zero");
    assert_eq!(merge_choice(1), "merged: positive");
}

#[test]
fn choice_merge_allows_terminal_siblings() {
    assert_eq!(partially_merge_choice(-1, "!".to_owned()), "NEGATIVE!");
    assert_eq!(partially_merge_choice(0, "!".to_owned()), "ZERO!");
    assert_eq!(partially_merge_choice(1, "!".to_owned()), "positive!");
}

#[contour]
fn lead_with_a_terminal_case(value: i32) -> String {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (negative, zero, positive) {
        match value {
            ..0 => "negative",
            0 => "zero",
            _ => "positive",
        }
    };

    #[action("Return the negative result.")]
    |negative| -> negative_result { negative.to_owned() };

    #[action("Build the zero result.")]
    |zero| -> zero_value { zero };

    #[action("Build the positive result.")]
    |positive| -> positive_value { positive };

    #[merge]
    |zero_value, positive_value| -> selected {};

    #[action("Uppercase the merged result.")]
    |selected| -> result { selected.to_uppercase() };
}

#[test]
fn choice_merge_allows_a_leading_terminal_sibling() {
    assert_eq!(lead_with_a_terminal_case(-1), "negative");
    assert_eq!(lead_with_a_terminal_case(0), "ZERO");
    assert_eq!(lead_with_a_terminal_case(1), "POSITIVE");
}
