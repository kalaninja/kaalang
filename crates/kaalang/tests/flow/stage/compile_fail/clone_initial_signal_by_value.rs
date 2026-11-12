use kaalang::kaa;

#[kaa]
fn clone_initial_signal_by_value(go: String) -> String {
    #[kaa::action("Clone by value.")]
    let copy = |go| go.clone();
    #[kaa::action("Use the clone.")]
    |copy| drop(copy);
    #[kaa::stage("Return the original.")]
    |go| {
        |go| return go;
    };
}

fn main() {}
