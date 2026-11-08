use kaalang::kaalang;

#[kaalang("An attachment.")]
struct Attachment<T> {
    #[refer(T, "The payload.")]
    payload: Box<T>,
}

fn main() {}
