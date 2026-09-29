use kaalang::kaalang;

#[kaalang]
fn clone_initial_signal_by_value(go: String) -> String {
    #[action("Clone by value.")]
    let copy = |go| go.clone();
    #[action("Use the clone.")]
    |copy| drop(copy);
    #[stage("Return the original.")]
    |go| {
        |go| return go;
    };
}

fn main() {}
