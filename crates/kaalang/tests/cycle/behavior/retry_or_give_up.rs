use kaalang::kaalang;

#[kaalang]
fn retry_or_give_up(mut attempts: u32, mut state: u8) -> Option<u8> {
    #[cycle("Poll until a response arrives.")]
    let result = loop {
        #[choice("What did the poll return?")]
        #[case("A response.")]
        #[case("A timeout.")]
        #[case("The server is busy.")]
        let (value, timeout, busy) = |&state| match *state {
            0 => 7,
            1 => (),
            _ => (),
        };

        #[question("Are the attempts exhausted?")]
        let (exhausted, retry) = |timeout, &mut attempts| {
            *attempts -= 1;
            *attempts == 0
        };

        #[action("Keep the response.")]
        let result = |value| Some(value);

        #[action("Give up.")]
        let result = |exhausted| None;

        #[action("Wait until the server is free.")]
        let again = |busy, &mut state| *state = 0;

        #[action("Poll again after the timeout.")]
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
