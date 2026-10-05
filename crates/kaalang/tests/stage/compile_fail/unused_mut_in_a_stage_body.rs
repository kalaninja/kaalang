#![deny(unused_mut)]

use kaalang::kaalang;

#[kaalang]
fn unused_mut_in_a_stage_body(start: ()) -> u8 {
    #[stage("Compute the value.")]
    let finish = |start| {
        #[action("Compute it with an unneeded `mut`.")]
        let finish = || {
            let mut value = 1;
            value
        };
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
