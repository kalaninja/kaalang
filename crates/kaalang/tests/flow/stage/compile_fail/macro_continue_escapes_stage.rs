use kaalang::kaa;

macro_rules! escape {
    () => {
        if false { continue }
    };
}

#[kaa]
fn macro_continue_escapes_stage(go: ()) {
    #[kaa::stage("Try to escape.")]
    let finish = |go| {
        #[kaa::action("Try to escape.")]
        let finish = |go| {
            escape!();
            go
        };
    };

    #[kaa::stage("Finish.")]
    |finish| {
        return;
    };
}

fn main() {}
