//! The same transitions as `state_machine.rs`, with an explicit dispatcher.

use kaalang::kaa;

enum State {
    First,
    Second,
    Finish,
}

#[kaa]
fn state_machine_without_stages(mut state: State) {
    #[kaa::cycle("Follow state transitions until `Finish`.")]
    let done = loop {
        #[kaa::choice("Which state is selected?")]
        #[kaa::case("First")]
        #[kaa::case("Second")]
        #[kaa::case("Finish")]
        let (first, second, done) = |&state| match *state {
            State::First => (),
            State::Second => (),
            State::Finish => (),
        };

        #[kaa::action("Continue to `Second`.")]
        let changed = |first, &mut state| *state = State::Second;

        #[kaa::action("Continue to `Finish`.")]
        let changed = |second, &mut state| *state = State::Finish;

        |changed| continue;
    };

    |done| return;
}

#[test]
fn reaches_finish() {
    state_machine_without_stages(State::First);
}
