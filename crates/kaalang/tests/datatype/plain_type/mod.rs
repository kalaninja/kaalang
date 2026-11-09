#![allow(dead_code)]

use kaalang::kaa;

// A plain type, and still a card: its structure without descriptions.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Timestamp {
    seconds: u64,
    nanos: u32,
}

#[kaa("A change saved to a document.")]
struct Change {
    #[kaa::refer(Timestamp, "When the change was saved.")]
    saved: Timestamp,
    #[kaa::value("What changed.")]
    summary: String,
}

#[test]
fn a_relationship_may_target_a_plain_type() {
    let saved = Timestamp {
        seconds: 60,
        nanos: 0,
    };
    let change = Change {
        saved,
        summary: String::from("Fixed a typo."),
    };

    assert_eq!(change.saved, saved);
}
