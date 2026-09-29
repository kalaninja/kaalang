use kaalang::kaalang;

#[kaalang]
fn nonexclusive_stage_producers(go: ()) -> u8 {
    #[stage("Produce twice.")]
    let finish = |go| {
        #[action("First.")]
        let finish = |go| 1u8;
        #[action("Second.")]
        let finish = || 2u8;
    };
    #[stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
