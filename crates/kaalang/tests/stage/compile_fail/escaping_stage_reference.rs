use kaalang::kaalang;

#[kaalang]
fn escaping_stage_reference<'a>(mut slot: &'a mut Option<&'a u8>, go: ()) {
    #[stage("Store a local reference.")]
    let finish = |go| {
        #[action("Create a local value.")]
        let local = |go| 7u8;

        #[action("Store its reference outside the visit.")]
        |&mut slot, &local| **slot = Some(local);

        #[action("Finish the visit.")]
        let finish = || ();
    };

    #[stage("Return.")]
    |finish| {
        return;
    };
}

fn main() {}
