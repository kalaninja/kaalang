#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A pool slot, free or holding a value, in shared storage.")]
union Slot {
    #[kaa::refer(Slot, "The next free slot, while this one is free.")]
    next: u32,
    #[kaa::value("The stored value, while this slot is taken.")]
    value: f32,
}

#[test]
fn a_union_may_refer_to_itself() {
    let slots = [Slot { next: 1 }, Slot { value: 0.5 }];
    assert_eq!(size_of_val(&slots), 2 * size_of::<u32>());
}
