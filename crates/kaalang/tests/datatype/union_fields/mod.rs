#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A member key or its raw number, in shared storage.")]
union Lookup {
    #[kaa::refer(Member, "The member the key names.")]
    id: MemberId,
    #[kaa::value("The key as a number.")]
    raw: u32,
}

#[kaa("A key in the member directory.")]
#[derive(Clone, Copy)]
struct MemberId(u32);

#[kaa("A member of the directory.")]
struct Member {
    #[kaa::value("The member's key.")]
    id: MemberId,
}

#[test]
fn a_union_keeps_its_shared_storage() {
    assert_eq!(size_of::<Lookup>(), size_of::<u32>());
}
