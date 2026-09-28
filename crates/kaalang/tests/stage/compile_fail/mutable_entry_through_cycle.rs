use kaalang::kaalang;

#[kaalang]
fn mutable_entry_through_cycle(go: u8) -> u8 {
    #[stage("Use the entry.")]
    let finish = |go| {
        #[cycle("Read the entry.")]
        let finish = |go| {
            #[action("Mutate the entry.")]
            let finish = |&mut go| {
                *go += 1;
                *go
            };
        };
    };

    #[stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
