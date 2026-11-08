use kaalang::kaa;

#[kaa("An attachment.")]
struct Attachment<T> {
    #[kaa::refer(T, "The payload.")]
    payload: Box<T>,
}

fn main() {}
