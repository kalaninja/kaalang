use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) {
    #[kaa::cycle("Try to return from a nested branch.")]
    |condition| loop {
        #[kaa::question("Return from this branch?")]
        let (leave, repeat) = |condition| condition;

        |leave| return;
        |repeat| continue;
    };
}

fn main() {}
