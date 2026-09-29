use kaalang::kaalang;

#[kaalang]
fn shadow_common_stage_wire(shared: u8, go: ()) -> u8 {
    #[stage("Shadow.")]
    let finish = |go| {
        #[action("Shadow shared data.")]
        let shared = || 1u8;

        #[action("Forward the local value.")]
        let finish = |shared| shared;
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
