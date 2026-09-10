#![deny(unused_mut)]

use kaalang::kaalang;

#[kaalang]
fn mutate_merged_branch_outputs(condition: bool, mode: bool) -> usize {
    #[question("Provide the value directly?")]
    let (mut selected, choose) = |condition| condition;

    #[choice("Choose the alternative producer.")]
    #[case("Provide the value directly.")]
    #[case("Produce it in an action.")]
    let (mut selected, prepare) = |choose, mode| match mode {
        true => (),
        false => (),
    };

    #[action("Produce the final alternative.")]
    let mut selected = |prepare| {};

    #[action("Mutate only after the branch outputs merge.")]
    let end = |&mut selected| {
        *selected = ();
        1
    };
}

#[test]
fn merged_branch_outputs_are_ordinary_mutable_data() {
    for (condition, mode) in [(true, true), (false, true), (false, false)] {
        assert_eq!(mutate_merged_branch_outputs(condition, mode), 1);
    }
}
