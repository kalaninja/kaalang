use kaalang::kaa;

#[kaa]
fn escaping_stage_reference<'a>(mut slot: &'a mut Option<&'a u8>, go: ()) {
    #[kaa::stage("Store a local reference.")]
    let finish = |go| {
        #[kaa::action("Create a local value.")]
        let local = |go| 7u8;

        #[kaa::action("Store its reference outside the visit.")]
        |&mut slot, &local| **slot = Some(local);

        #[kaa::action("Finish the visit.")]
        let finish = || ();
    };

    #[kaa::stage("Return.")]
    |finish| {
        return;
    };
}

fn main() {}
