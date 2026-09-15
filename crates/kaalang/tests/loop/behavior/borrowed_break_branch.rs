use kaalang::kaalang;

/// A break may borrow the branch output that reaches it: the transfer is the
/// branch's continuation whether it consumes the output or only reads it.
#[kaalang]
fn borrowed_break_branch(flag: bool) {
    #[cycle("Check the flag.")]
    |flag| {
        #[question("Exit?")]
        let (done, _again) = |flag| flag;

        |&done| break;
    };

    return;
}

#[test]
fn a_break_may_borrow_its_branch_output() {
    borrowed_break_branch(true);
}
