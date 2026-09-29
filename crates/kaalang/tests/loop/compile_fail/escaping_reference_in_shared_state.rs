use kaalang::kaalang;

#[kaalang]
fn escaping_reference_in_shared_state<'a>(mut slot: &'a mut Option<&'a u8>) {
    #[cycle("Store a departing local in shared state.")]
    let finished = {
        #[action("Create a local owner.")]
        let local = || 7u8;

        #[action("Store its reference outside the iteration.")]
        |&mut slot, &local| **slot = Some(local);

        #[action("Complete the iteration.")]
        let finished = || ();
    };

    |finished| return;
}

fn main() {}
