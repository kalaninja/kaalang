#![allow(dead_code)]

use kaalang::kaa;

#[kaa("Overlapping word and byte views of shared storage.")]
#[repr(C, align(16))]
#[derive(Clone, Copy)]
union Payload {
    #[kaa::value("The payload as a word.")]
    word: u64,
    #[kaa::value("The payload as bytes.")]
    bytes: [u8; 8],
}

#[kaa("A selected payload, by its index.")]
#[repr(transparent)]
struct Selection {
    #[kaa::refer(Payload, "The selected payload, resolved by index.")]
    index: usize,
}

#[kaa("A compact handle to a payload.")]
#[repr(C, packed)]
struct Handle {
    #[kaa::refer(Payload, "The payload in this slot.")]
    slot: u16,
    #[kaa::value("The generation of the slot.")]
    generation: u8,
}

#[test]
fn representation_attributes_pass_through() {
    assert_eq!(align_of::<Payload>(), 16);
    assert_eq!(size_of::<Handle>(), 3);
    assert_eq!(size_of::<Selection>(), size_of::<usize>());
}
