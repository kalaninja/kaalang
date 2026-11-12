use kaalang::kaa;

#[kaa]
fn escaping_reference_in_shared_state<'a>(mut slot: &'a mut Option<&'a u8>) {
    #[kaa::cycle("Store a departing local in shared state.")]
    let finished = loop {
        #[kaa::action("Create a local owner.")]
        let local = || 7u8;

        #[kaa::action("Store its reference outside the iteration.")]
        |&mut slot, &local| **slot = Some(local);

        #[kaa::action("Complete the iteration.")]
        let finished = || ();
    };

    |finished| return;
}

fn main() {}
