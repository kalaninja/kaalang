use kaalang::kaa;

#[kaa("Who may approve a change.")]
enum Approval {
    #[kaa::case]
    Open,
}

fn main() {}
