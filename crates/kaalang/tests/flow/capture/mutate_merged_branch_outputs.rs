#![deny(unused_mut)]

use kaalang::kaa;

#[kaa]
fn mutate_merged_branch_outputs(condition: bool, mode: bool) -> usize {
    #[kaa::question("Provide the value directly?")]
    let (mut selected, choose) = |condition| condition;

    #[kaa::choice("Choose the alternative producer.")]
    #[kaa::case("Provide the value directly.")]
    #[kaa::case("Produce it in an action.")]
    let (mut selected, prepare) = |choose, mode| match mode {
        true => (),
        false => (),
    };

    #[kaa::action("Produce the final alternative.")]
    let mut selected = |prepare| {};

    #[kaa::action("Mutate only after the branch outputs merge.")]
    let end = |&mut selected| {
        *selected = ();
        1
    };

    |end| return end;
}

#[test]
fn merged_branch_outputs_are_ordinary_mutable_data() {
    for (condition, mode) in [(true, true), (false, true), (false, false)] {
        assert_eq!(mutate_merged_branch_outputs(condition, mode), 1);
    }
}
