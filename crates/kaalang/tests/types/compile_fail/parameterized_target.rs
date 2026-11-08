use kaalang::kaalang;

#[kaalang("An attachment.")]
struct Attachment<T>(T);

#[kaalang("An upload.")]
struct Upload {
    #[refer(Attachment<u8>, "The uploaded content.")]
    content: Attachment<u8>,
}

fn main() {}
