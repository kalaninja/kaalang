use kaalang::kaa;

#[kaa("A publication state.")]
enum State {
    #[kaa::case("Not yet published.")]
    #[kaa::case("Still a draft.")]
    Draft,
}

fn main() {}
