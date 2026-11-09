#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A payload of raw bytes.")]
struct Payload {
    #[kaa::value("The payload bytes.")]
    bytes: Vec<u8>,
}

#[kaa("An attachment with a configurable payload.")]
struct Attachment<T> {
    #[kaa::value("The boxed attachment payload.")]
    payload: Box<T>,
    #[kaa::value("A digest of the payload.")]
    digest: [u8; 4096],
}

// `Payload` in the field type is storage, not a relationship: only
// `Attachment` gets a line.
#[kaa("An upload waiting for review.")]
struct Upload {
    #[kaa::refer(Attachment, "The uploaded content.")]
    content: Attachment<Payload>,
}

#[test]
fn a_generic_target_is_named_without_its_parameters() {
    let upload = Upload {
        content: Attachment {
            payload: Box::new(Payload { bytes: vec![1, 2] }),
            digest: [0; 4096],
        },
    };

    assert_eq!(upload.content.payload.bytes, [1, 2]);
}
