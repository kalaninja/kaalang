use kaalang::kaa;

#[kaa]
fn shadow_common_stage_wire(shared: u8, go: ()) -> u8 {
    #[kaa::stage("Shadow.")]
    let finish = |go| {
        #[kaa::action("Shadow shared data.")]
        let shared = || 1u8;

        #[kaa::action("Forward the local value.")]
        let finish = |shared| shared;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
