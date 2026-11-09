use kaalang::kaa;

#[kaa]
fn single_terminal_stage_without_preparation(go: u8) -> u8 {
    #[kaa::stage("Increment the value.")]
    |go| {
        #[kaa::action("Add one.")]
        let finish = |go| go + 1;
        |finish| return finish;
    };
}

fn main() {}
