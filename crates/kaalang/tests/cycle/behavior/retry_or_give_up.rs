use kaalang::kaa;

#[kaa]
fn retry_or_give_up(mut attempts: u32, mut state: u8) -> Option<u8> {
    #[kaa::cycle("Poll until a response arrives.")]
    let result = loop {
        #[kaa::choice("What did the poll return?")]
        #[kaa::case("A response.")]
        #[kaa::case("A timeout.")]
        #[kaa::case("The server is busy.")]
        let (value, timeout, busy) = |&state| match *state {
            0 => 7,
            1 => (),
            _ => (),
        };

        #[kaa::question("Are the attempts exhausted?")]
        let (exhausted, retry) = |timeout, &mut attempts| {
            *attempts -= 1;
            *attempts == 0
        };

        #[kaa::action("Keep the response.")]
        let result = |value| Some(value);

        #[kaa::action("Give up.")]
        let result = |exhausted| None;

        #[kaa::action("Wait until the server is free.")]
        let again = |busy, &mut state| *state = 0;

        #[kaa::action("Poll again after the timeout.")]
        let again = |retry| {};

        |again| continue;
    };

    |result| return result;
}

#[test]
fn one_case_can_either_complete_or_repeat_the_cycle() {
    assert_eq!(retry_or_give_up(3, 0), Some(7));
    assert_eq!(retry_or_give_up(3, 2), Some(7));
    assert_eq!(retry_or_give_up(3, 1), None);
    assert_eq!(retry_or_give_up(1, 1), None);
}
