#[contour]
fn route(request: u8) -> u8 {
    #[question("Is there an application, and is it eligible?")]
    |&request| -> (accepted, rejected) { request > 0 };

    #[choice("Which path should process this application?")]
    #[case("Short path")]
    #[case("Long path with an additional check")]
    |accepted, &request| -> (short, long) {
        match request {
            1 => (),
            _ => (),
        }
    };

    #[action("Prepare the short result.")]
    |short, &request| -> short_value { request };

    #[action("Prepare the long result while preserving every important application detail.")]
    |long, &request| -> long_value { request };

    #[merge]
    |short_value, long_value| -> selected {};

    #[action("Return the selected result.")]
    |selected| -> result { selected };

    #[action("Reject the application.")]
    |rejected, request| -> declined { request };
}
