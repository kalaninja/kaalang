use kaalang::kaa;

// The transform uses only common inputs while the branches of the question are
// still separate. Nonempty input and output lists do not make it branch-local.
#[kaa]
fn invalid(condition: bool, value: String) -> usize {
    #[kaa::question("Log the value?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Log.")]
    let logged = |yes, &value| {
        assert_eq!(value, "abc");
    };

    #[kaa::action("Transform.")]
    let length = |value| { value.len() };

    #[kaa::action("Finish yes.")]
    let end = |logged, length| { length };

    #[kaa::action("Finish no.")]
    let end = |no, length| { length };

    |end| return end;
}

fn main() {}
