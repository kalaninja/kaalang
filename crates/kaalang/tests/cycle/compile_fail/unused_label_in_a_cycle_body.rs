#![deny(unused_labels)]

use kaalang::kaa;

#[kaa]
fn unused_label_in_a_cycle_body(limit: u32) -> u32 {
    #[kaa::cycle("Take the limit once.")]
    let total = loop {
        #[kaa::action("Take it through a labeled block.")]
        let total = |limit| {
            'taken: {
                limit
            }
        };
    };

    |total| return total;
}

fn main() {}
