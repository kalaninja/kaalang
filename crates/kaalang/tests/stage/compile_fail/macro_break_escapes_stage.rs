use kaalang::kaa;

macro_rules! escape {
    () => {
        if false { break }
    };
}

#[kaa]
fn macro_break_escapes_stage(go: ()) {
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
