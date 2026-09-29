use kaalang::kaalang;

#[kaalang]
fn return_in_preparation(go: ()) {
    return;
    #[stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
