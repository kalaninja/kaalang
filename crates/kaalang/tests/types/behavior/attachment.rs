#![allow(dead_code)]

use kaalang::kaa;

#[kaa("Overlapping word and byte views of shared storage.")]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
union Payload {
    #[kaa::value("The payload as a word.")]
    word: u64,
    #[kaa::value("The payload as bytes.")]
    bytes: [u8; 8],
}

#[kaa("An attachment with a configurable payload.")]
#[repr(C)]
struct Attachment<T> {
    #[kaa::value("The boxed attachment payload.")]
    payload: Box<T>,
    #[kaa::value("A digest of the payload.")]
    digest: [u8; 4096],
}

#[kaa("An upload waiting for review.")]
struct Upload {
    #[kaa::refer(Attachment, "The uploaded content.")]
    content: Attachment<Payload>,
    #[kaa::refer(crate::behavior::attachment::Payload, "The checksum of the content.")]
    checksum: Payload,
}

#[kaa("A selected attachment, by its index.")]
#[repr(transparent)]
struct Selection {
    #[kaa::refer(Attachment, "The selected attachment, resolved by index.")]
    index: usize,
}

#[kaa("A compact handle to an attachment.")]
#[repr(C, packed)]
struct Handle {
    #[kaa::refer(Attachment, "The attachment in this slot.")]
    slot: u16,
    #[kaa::value("The generation of the slot.")]
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
