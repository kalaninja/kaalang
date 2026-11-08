use kaalang::kaalang;

#[kaalang("A publication state.")]
enum State {
    #[case("Not yet published.")]
    #[case("Still a draft.")]
    Draft,
}

fn main() {}
