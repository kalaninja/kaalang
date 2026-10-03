use kaalang::kaalang;

#[kaalang]
fn nested_cycle_shadows_stage_entry(go: ()) {
    #[stage("Prepare a local result.")]
    let other = |go| {
        #[cycle("Prepare a local result.")]
        let other = loop {
            #[action("Shadow the entry.")]
            let go = || ();

            #[action("Finish locally.")]
            let other = |go| ();
        };
    };

    #[stage("Finish.")]
    |other| {
        |other| return;
    };
}

fn main() {}
