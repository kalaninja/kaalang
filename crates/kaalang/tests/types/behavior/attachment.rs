#![allow(dead_code)]

use kaalang::kaalang;

#[kaalang("Overlapping word and byte views of shared storage.")]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
union Payload {
    #[value("The payload as a word.")]
    word: u64,
    #[value("The payload as bytes.")]
    bytes: [u8; 8],
}

#[kaalang("An attachment with a configurable payload.")]
#[repr(C)]
struct Attachment<T> {
    #[value("The boxed attachment payload.")]
    payload: Box<T>,
    #[value("A digest of the payload.")]
    digest: [u8; 4096],
}

#[kaalang("An upload waiting for review.")]
struct Upload {
    #[refer(Attachment, "The uploaded content.")]
    content: Attachment<Payload>,
    #[refer(crate::behavior::attachment::Payload, "The checksum of the content.")]
    checksum: Payload,
}

#[kaalang("A selected attachment, by its index.")]
#[repr(transparent)]
struct Selection {
    #[refer(Attachment, "The selected attachment, resolved by index.")]
    index: usize,
}

#[kaalang("A compact handle to an attachment.")]
#[repr(C, packed)]
struct Handle {
    #[refer(Attachment, "The attachment in this slot.")]
    slot: u16,
    #[value("The generation of the slot.")]
    generation: u8,
}

#[test]
fn representation_attributes_pass_through() {
    let upload = Upload {
        content: Attachment {
            payload: Box::new(Payload { word: 0 }),
            digest: [0; 4096],
        },
        checksum: Payload { bytes: [0; 8] },
    };

    assert_eq!(align_of::<Payload>(), 8);
    assert_eq!(size_of::<Handle>(), 3);
    assert_eq!(size_of::<Selection>(), size_of::<usize>());
    assert_eq!(size_of_val(&upload), size_of::<Attachment<Payload>>() + 8);
}
