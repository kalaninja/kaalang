use kaalang::kaalang;

#[kaalang]
fn capture_other_stage_entry(first: ()) {
    #[stage("First.")]
    let second = |first| {
        #[action("Select second.")]
        let second = |first| ();
    };
    #[stage("Second.")]
    |second| {
        #[action("Read the earlier entry.")]
        |first| ();
        return;
    };
}

fn main() {}
