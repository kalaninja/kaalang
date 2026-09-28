use kaalang::kaalang;

#[kaalang]
fn single_terminal_stage_without_preparation(go: u8) -> u8 {
    #[stage("Increment the value.")]
    |go| {
        #[action("Add one.")]
        let finish = |go| go + 1;
        |finish| return finish;
    };
}

fn main() {}
