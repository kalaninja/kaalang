use kaalang::kaa;

#[kaa]
fn nested_cycle_shadows_stage_entry(go: ()) {
    #[kaa::stage("Prepare a local result.")]
    let other = |go| {
        #[kaa::cycle("Prepare a local result.")]
        let other = loop {
            #[kaa::action("Shadow the entry.")]
            let go = || ();

            #[kaa::action("Finish locally.")]
            let other = |go| ();
        };
    };

    #[kaa::stage("Finish.")]
    |other| {
        |other| return;
    };
}

fn main() {}
