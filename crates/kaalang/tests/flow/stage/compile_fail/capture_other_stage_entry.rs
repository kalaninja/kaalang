use kaalang::kaa;

#[kaa]
fn capture_other_stage_entry(first: ()) {
    #[kaa::stage("First.")]
    let second = |first| {
        #[kaa::action("Select second.")]
        let second = |first| ();
    };
    #[kaa::stage("Second.")]
    |second| {
        #[kaa::action("Read the earlier entry.")]
        |first| ();
        return;
    };
}

fn main() {}
