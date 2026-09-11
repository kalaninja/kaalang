use kaalang::kaalang;

#[kaalang]
const fn empty_captures() -> usize {
    || loop {
        || {
            break;
        };
    };

    loop {
        break;
    }

    #[action("Continue after both loops.")]
    let end = || 7;
}

#[test]
fn empty_capture_lists_and_bare_breaks_leave_normally() {
    const SEVEN: usize = empty_captures();
    assert_eq!(SEVEN, 7);
}
