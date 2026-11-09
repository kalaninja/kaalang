//! A minimal state machine using stages; compare `state_machine_without_stages.rs`.

use kaalang::kaa;

#[kaa]
fn state_machine(first: ()) {
    #[kaa::stage("First")]
    let second = |first| {
        #[kaa::action("Continue to `Second`.")]
        let second = || {};
    };

    #[kaa::stage("Second")]
    let finish = |second| {
        #[kaa::action("Continue to `Finish`.")]
        let finish = || {};
    };

    #[kaa::stage("Finish")]
    |finish| {
        return;
    };
}

#[test]
fn reaches_finish() {
    state_machine(());
}
