use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) {
    #[cycle("Try to return from a nested branch.")]
    |condition| {
        #[question("Return from this branch?")]
        let (leave, repeat) = |condition| condition;

        |leave| return;
        |repeat| break;
    };
}

fn main() {}
