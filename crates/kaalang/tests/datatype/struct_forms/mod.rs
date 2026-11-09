#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A member of the directory.")]
pub struct Member {
    #[kaa::value("The member's key.")]
    pub id: MemberId,
    #[kaa::value("The member's display name.")]
    pub(crate) name: String,
    // No annotation: the tile shows the field's name and type only.
    active: bool,
    #[kaa::refer(Departed, "Set once the member leaves.")]
    pub departed: Option<Departed>,
}

#[kaa("A key in the member directory.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemberId(pub u32);

#[kaa("A marker that a member has left the directory.")]
pub struct Departed;

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
