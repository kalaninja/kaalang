use kaalang::kaa;

#[kaa]
fn return_in_preparation(go: ()) {
    return;
    #[kaa::stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
