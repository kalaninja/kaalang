#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A key in the member directory.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MemberId(u32);

#[kaa("A marker that a member has left the directory.")]
struct Departed;

#[kaa("A member of the directory.")]
struct Member {
    #[kaa::value("The member's key.")]
    id: MemberId,
    #[kaa::value("The member's display name.")]
    name: String,
    // No annotation: the tile shows the field's name and type only.
    active: bool,
    #[kaa::refer(Departed, "Set once the member leaves.")]
    departed: Option<Departed>,
}

#[test]
fn named_tuple_and_unit_structs_keep_their_forms() {
    let member = Member {
        id: MemberId(7),
        name: String::from("Sam"),
        active: false,
        departed: Some(Departed),
    };

    assert_eq!(member.id, MemberId(7));
    assert!(!member.active && member.departed.is_some());
}
