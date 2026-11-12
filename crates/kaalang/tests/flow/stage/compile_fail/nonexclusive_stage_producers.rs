use kaalang::kaa;

#[kaa]
fn nonexclusive_stage_producers(go: ()) -> u8 {
    #[kaa::stage("Produce twice.")]
    let finish = |go| {
        #[kaa::action("First.")]
        let finish = |go| 1u8;
        #[kaa::action("Second.")]
        let finish = || 2u8;
    };
    #[kaa::stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
