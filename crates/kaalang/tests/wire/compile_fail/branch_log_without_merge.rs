use kaalang::kaalang;

// The transform uses only common inputs while the branches of the question are
// still separate. Nonempty input and output lists do not make it branch-local.
#[kaalang]
fn invalid(condition: bool, value: String) -> usize {
    #[question("Log the value?")]
    let (yes, no) = |condition| { condition };

    #[action("Log.")]
    let logged = |yes, &value| {
        assert_eq!(value, "abc");
    };

    #[action("Transform.")]
    let length = |value| { value.len() };

    #[action("Finish yes.")]
    let end = |logged, length| { length };

    #[action("Finish no.")]
    let end = |no, length| { length };

    |end| return end;
}

fn main() {}
