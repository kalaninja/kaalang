use kaalang::kaalang;

#[kaalang]
const fn empty_captures() -> usize {
    #[cycle("Leave through an explicit empty capture list.")]
    {
        || break;
    }

    #[cycle("Leave through a bare break.")]
    || {
        break;
    };

    #[action("Continue after both loops.")]
    let result = || 7;

    |result| return result;
}

#[test]
fn zero_interface_shorthand_and_bare_breaks_leave_normally() {
    const SEVEN: usize = empty_captures();
    assert_eq!(SEVEN, 7);
}
