//! A minimal state machine using stages; compare `state_machine_without_stages.rs`.

use kaalang::kaalang;

#[kaalang]
fn state_machine(first: ()) {
    #[stage("First")]
    let second = |first| {
        #[action("Continue to `Second`.")]
        let second = || {};
    };

    #[stage("Second")]
    let finish = |second| {
        #[action("Continue to `Finish`.")]
        let finish = || {};
    };

    #[stage("Finish")]
    |finish| {
        return;
    };
}

#[test]
fn reaches_finish() {
    state_machine(());
}
