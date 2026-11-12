use kaalang::kaa;

#[kaa]
const fn select_value<T: Copy>(take_left: bool, left: T, right: T) -> T {
    #[kaa::action("Begin selection.")]
    let choose = || {};

    #[kaa::stage("Select one value.")]
    let finish = |choose| {
        #[kaa::question("Take the left value?")]
        let (yes, no) = |take_left| take_left;

        #[kaa::action("Select left.")]
        let finish = |yes, left| left;

        #[kaa::action("Select right.")]
        let finish = |no, right| right;
    };

    #[kaa::stage("Return the selected value.")]
    |finish| {
        |finish| return finish;
    };
}

const _: () = assert!(select_value(true, 3u8, 4u8) == 3);

#[test]
fn merges_alternative_transition_values() {
    assert_eq!(select_value(false, (1, 2), (3, 4)), (3, 4));
}
