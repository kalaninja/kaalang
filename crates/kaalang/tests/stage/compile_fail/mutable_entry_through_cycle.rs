use kaalang::kaa;

#[kaa]
fn mutable_entry_through_cycle(go: u8) -> u8 {
    #[kaa::stage("Use the entry.")]
    let finish = |go| {
        #[kaa::cycle("Read the entry.")]
        let finish = |go| loop {
            #[kaa::action("Mutate the entry.")]
            let finish = |&mut go| {
                *go += 1;
                *go
            };
        };
    };

    #[kaa::stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
