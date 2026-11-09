use kaalang::kaa;

#[kaa("Who may approve a change.")]
#[kaa::case("Anyone may approve.")]
enum Approval {
    Open,
}

fn main() {}
