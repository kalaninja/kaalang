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
    |short, &request| -> selected { request };

    #[action("Prepare the long result while preserving every important application detail.")]
    |long, &request| -> selected { request };

    #[action("Return the selected result.")]
    |selected| -> result { selected };

    #[action("Reject the application.")]
    |rejected, request| -> result { request };

    #[end]
    |result| {};
}
