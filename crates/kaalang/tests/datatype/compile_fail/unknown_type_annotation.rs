use kaalang::kaa;

#[kaa("A member of the workspace.")]
struct Member {
    #[kaa::vlaue("The display name.")]
    name: String,
}

fn main() {}
