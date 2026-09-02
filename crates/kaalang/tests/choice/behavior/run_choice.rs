use kaalang::kaalang;

#[kaalang]
fn run_choice(value: i32, branch_action_count: &mut usize) -> &'static str {
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

    #[action("Produce the negative result.")]
    |negative, branch_action_count| -> result {
        *branch_action_count += 1;
        "negative"
    };

    #[action("Produce the zero result.")]
    |zero, branch_action_count| -> result {
        *branch_action_count += 1;
        "zero"
    };

    #[action("Produce the positive result.")]
    |positive, branch_action_count| -> result {
        *branch_action_count += 1;
        "positive"
    };

    #[end]
    |result| {};
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
