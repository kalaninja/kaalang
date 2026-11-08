use kaalang::kaa;

#[kaa]
fn run_choice(value: i32, branch_action_count: &mut usize) -> &'static str {
    #[kaa::choice("What is the sign of the value?")]
    #[kaa::case("The value is negative.")]
    #[kaa::case("The value is zero.")]
    #[kaa::case("The value is positive.")]
    let (negative, zero, positive) = |value| match value {
        ..0 => (),
        0 => (),
        _ => (),
    };

    #[kaa::action("Produce the negative result.")]
    let end = |negative, branch_action_count| {
        *branch_action_count += 1;
        "negative"
    };

    #[kaa::action("Produce the zero result.")]
    let end = |zero, branch_action_count| {
        *branch_action_count += 1;
        "zero"
    };

    #[kaa::action("Produce the positive result.")]
    let end = |positive, branch_action_count| {
        *branch_action_count += 1;
        "positive"
    };

    |end| return end;
}

#[test]
fn choice_executes_each_branch() {
    let mut branch_action_count = 0;

    assert_eq!(run_choice(-1, &mut branch_action_count), "negative");
    assert_eq!(branch_action_count, 1);
    assert_eq!(run_choice(0, &mut branch_action_count), "zero");
    assert_eq!(branch_action_count, 2);
    assert_eq!(run_choice(1, &mut branch_action_count), "positive");
    assert_eq!(branch_action_count, 3);
    assert_eq!(run_choice(2, &mut branch_action_count), "positive");
    assert_eq!(branch_action_count, 4);
}
