use kaalang::kaa;

#[kaa("A publication state.")]
enum State {
    #[kaa::value("Not yet published.")]
    Draft,
}

fn main() {}
