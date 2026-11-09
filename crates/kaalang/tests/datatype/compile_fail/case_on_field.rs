use kaalang::kaa;

#[kaa("A member.")]
struct Member {
    #[kaa::case("The member's name.")]
    name: String,
}

fn main() {}
