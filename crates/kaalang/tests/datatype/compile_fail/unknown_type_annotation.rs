use kaalang::kaa;

#[kaa("A member of the workspace.")]
struct Member {
    #[kaa::vlaue("The display name.")]
    name: String,
}

// The type stands beside the error, so its uses still compile.
fn main() {
    let member = Member {
        name: String::new(),
    };
    drop(member.name);
}
