use kaalang::kaalang;

#[kaalang]
fn nested_cycle_shadows_outer_wire(value: u8) -> u8 {
    #[cycle("Complete an outer cycle.")]
    let result = {
        #[cycle("Try to shadow the outer value.")]
        let nested = {
            #[action("Reuse the outer name.")]
            let value = || 1u8;

            #[action("Complete the inner cycle.")]
            let nested = |value| ();
        };

        #[action("Complete the outer cycle.")]
        let result = |nested| 2u8;
    };

    |result| return result;
}

fn main() {}
