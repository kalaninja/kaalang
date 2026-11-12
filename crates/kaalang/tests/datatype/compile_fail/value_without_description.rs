use kaalang::kaa;

#[kaa("A member.")]
struct Member {
    #[kaa::value]
    name: String,
}

fn main() {}
