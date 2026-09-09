use kaalang::kaalang;

// The transform uses only common inputs while the branches of the question are
// still separate. Nonempty input and output lists do not make it branch-local.
#[kaalang]
fn invalid(condition: bool, value: String) -> usize {
    #[question("Log the value?")]
    |condition| -> (yes, no) { condition };

    #[action("Log.")]
    |yes, &value| -> logged {
        assert_eq!(value, "abc");
    };

    #[action("Transform.")]
    |value| -> length { value.len() };

    #[action("Finish yes.")]
    |logged, length| -> result { length };

    #[action("Finish no.")]
    |no, length| -> result { length };
}

fn main() {}
