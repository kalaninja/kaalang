use kaalang::kaalang;

macro_rules! escape {
    () => {
        if false { continue }
    };
}

#[kaalang]
fn macro_continue_escapes_stage(go: ()) {
    #[stage("Try to escape.")]
    let finish = |go| {
        #[action("Try to escape.")]
        let finish = |go| {
            escape!();
            go
        };
    };

    #[stage("Finish.")]
    |finish| {
        return;
    };
}

fn main() {}
