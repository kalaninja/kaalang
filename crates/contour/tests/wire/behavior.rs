use contour::contour;

#[contour]
fn question_value(condition: bool, shared_runs: &mut usize) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value.")]
    |yes| -> selected { 11 };

    #[action("Build the no value.")]
    |no| -> selected { 29 };

    #[action("Use the selected value once.")]
    |selected, shared_runs| -> result {
        *shared_runs += 1;
        selected
    };
}

#[contour]
fn choice_value(case: u8) -> &'static str {
    #[choice("Choose one of three values.")]
    #[case("First")]
    #[case("Second")]
    #[case("Third")]
    |case| -> (first, second, third) {
        match case {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { "first" };

    #[action("Build the second value.")]
    |second| -> selected { "second" };

    #[action("Build the third value.")]
    |third| -> selected { "third" };

    #[action("Use the selected value.")]
    |selected| -> result { selected };
}

#[contour]
fn convergence_with_terminal_sibling(case: u8) -> u32 {
    #[choice("Choose whether to continue.")]
    #[case("First continuing path")]
    #[case("Second continuing path")]
    #[case("Terminal path")]
    |case| -> (first, second, done) {
        match case {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { 1 };

    #[action("Build the second value.")]
    |second| -> selected { 2 };

    #[action("Finish the terminal path.")]
    |done| -> done_result { 99 };

    #[action("Use a value from a continuing path.")]
    |selected| -> result { selected * 10 };
}

#[contour]
fn several_wires(condition: bool) -> (u32, &'static str) {
    #[question("Choose a pair.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes pair.")]
    |yes| -> (number, label) { (1, "yes") };

    #[action("Build the no pair.")]
    |no| -> (number, label) { (2, "no") };

    #[action("Use both selected values.")]
    |label, number| -> result { (number, label) };
}

#[contour]
fn borrowed_common(condition: bool, prefix: String) -> String {
    #[question("Choose a suffix.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes suffix.")]
    |yes, &prefix| -> suffix { format!("{prefix}-yes") };

    #[action("Build the no suffix.")]
    |no, &prefix| -> suffix { format!("{prefix}-no") };

    #[action("Use the suffix and the preserved prefix.")]
    |suffix, prefix| -> result { format!("{prefix}:{suffix}") };
}

#[contour]
fn nested_convergence(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested path?")]
    |outer| -> (nested, direct) { outer };

    #[question("Choose the nested value.")]
    |nested, inner| -> (inner_yes, inner_no) { inner };

    #[action("Build the nested yes value.")]
    |inner_yes| -> selected { 1 };

    #[action("Build the nested no value.")]
    |inner_no| -> selected { 2 };

    #[action("Build the direct value.")]
    |direct| -> selected { 3 };

    #[action("Use the selected nested value.")]
    |selected| -> result { selected * 10 };
}

#[contour]
fn uneven_depth(condition: bool) -> u32 {
    #[question("Choose a path depth.")]
    |condition| -> (short, long) { condition };

    #[action("Build the short value.")]
    |short| -> selected { 5 };

    #[action("Prepare the long value.")]
    |long| -> prepared { 7 };

    #[action("Build the long value.")]
    |prepared| -> selected { prepared + 1 };

    #[action("Use the selected value.")]
    |selected| -> result { selected * 2 };
}

#[test]
fn question_converges_two_producers_into_one_consumer() {
    let mut shared_runs = 0;
    assert_eq!(question_value(true, &mut shared_runs), 11);
    assert_eq!(shared_runs, 1);
    assert_eq!(question_value(false, &mut shared_runs), 29);
    assert_eq!(shared_runs, 2);
}

#[test]
fn choice_converges_three_producers() {
    assert_eq!(choice_value(0), "first");
    assert_eq!(choice_value(1), "second");
    assert_eq!(choice_value(2), "third");
}

#[test]
fn terminal_siblings_are_excluded_from_convergence() {
    assert_eq!(convergence_with_terminal_sibling(0), 10);
    assert_eq!(convergence_with_terminal_sibling(1), 20);
    assert_eq!(convergence_with_terminal_sibling(2), 99);
}

#[test]
fn convergence_carries_several_wires_in_consumer_order() {
    assert_eq!(several_wires(true), (1, "yes"));
    assert_eq!(several_wires(false), (2, "no"));
}

#[test]
fn borrowed_common_wire_survives_convergence() {
    assert_eq!(borrowed_common(true, "root".into()), "root:root-yes");
    assert_eq!(borrowed_common(false, "root".into()), "root:root-no");
}

#[test]
fn nested_paths_converge_once() {
    assert_eq!(nested_convergence(true, true), 10);
    assert_eq!(nested_convergence(true, false), 20);
    assert_eq!(nested_convergence(false, true), 30);
}

#[test]
fn branches_may_reach_convergence_at_different_depths() {
    assert_eq!(uneven_depth(true), 10);
    assert_eq!(uneven_depth(false), 16);
}
