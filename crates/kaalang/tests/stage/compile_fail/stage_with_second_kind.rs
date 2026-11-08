use kaalang::kaa;

#[kaa]
fn stage_with_second_kind(go: ()) {
    #[kaa::stage("Finish.")]
    #[kaa::action("Other.")]
    |go| {
        return;
    };
}

fn main() {}
