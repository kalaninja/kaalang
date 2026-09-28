use kaalang::kaalang;

#[kaalang]
fn stage_with_second_kind(go: ()) {
    #[stage("Finish.")]
    #[action("Other.")]
    |go| {
        return;
    };
}

fn main() {}
