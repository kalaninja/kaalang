use kaalang::kaa;

#[kaa("An attachment.")]
struct Attachment<T>(T);

#[kaa("An upload.")]
struct Upload {
    #[kaa::refer(Attachment<u8>, "The uploaded content.")]
    content: Attachment<u8>,
}

fn main() {}
