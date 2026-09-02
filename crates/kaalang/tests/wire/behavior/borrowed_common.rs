use kaalang::kaalang;

#[kaalang]
fn borrowed_common(condition: bool, prefix: String) -> String {
    #[question("Choose a suffix.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes suffix.")]
    |yes, &prefix| -> suffix { format!("{prefix}-yes") };

    #[action("Build the no suffix.")]
    |no, &prefix| -> suffix { format!("{prefix}-no") };

    #[action("Use the suffix and the preserved prefix.")]
    |suffix, prefix| -> result { format!("{prefix}:{suffix}") };

    #[end]
    |result| {};
}

#[test]
fn borrowed_common_wire_survives_convergence() {
    assert_eq!(borrowed_common(true, "root".into()), "root:root-yes");
    assert_eq!(borrowed_common(false, "root".into()), "root:root-no");
}
