#![deny(unused_mut)]

use kaalang::kaa;

#[kaa]
fn unused_mut_in_a_stage_body(start: ()) -> u8 {
    #[kaa::stage("Compute the value.")]
    let finish = |start| {
        #[kaa::action("Compute it with an unneeded `mut`.")]
        let finish = || {
            let mut value = 1;
            value
        };
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
