use kaalang::kaa;

#[kaa]
fn nested_cycle_shadows_outer_wire(value: u8) -> u8 {
    #[kaa::cycle("Complete an outer cycle.")]
    let result = loop {
        #[kaa::cycle("Try to shadow the outer value.")]
        let nested = loop {
            #[kaa::action("Reuse the outer name.")]
            let value = || 1u8;

            #[kaa::action("Complete the inner cycle.")]
            let nested = |value| ();
        };

        #[kaa::action("Complete the outer cycle.")]
        let result = |nested| 2u8;
    };

    |result| return result;
}

fn main() {}
