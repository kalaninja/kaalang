//! The same transitions as `state_machine.rs`, with an explicit dispatcher.

use kaalang::kaalang;

enum State {
    First,
    Second,
    Finish,
}

#[kaalang]
fn state_machine_without_stages(mut state: State) {
    #[cycle("Follow state transitions until `Finish`.")]
    let done = {
        #[choice("Which state is selected?")]
        #[case("First")]
        #[case("Second")]
        #[case("Finish")]
        let (first, second, done) = |&state| match *state {
            State::First => (),
            State::Second => (),
            State::Finish => (),
        };

        #[action("Continue to `Second`.")]
        let changed = |first, &mut state| *state = State::Second;

        #[action("Continue to `Finish`.")]
        let changed = |second, &mut state| *state = State::Finish;

        |changed| continue;
    };

    |done| return;
}

#[test]
fn reaches_finish() {
    state_machine_without_stages(State::First);
}
