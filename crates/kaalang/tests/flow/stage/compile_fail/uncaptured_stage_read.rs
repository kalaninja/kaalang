use kaalang::kaa;

#[kaa]
fn uncaptured_stage_read(go: u8) -> u8 {
    #[kaa::stage("Read.")]
    let answer = |go| {
        #[kaa::action("Read without a capture.")]
        let answer = || go + 1;
    };

    #[kaa::stage("Return the result.")]
    |answer| {
        |answer| return answer;
    };
}

fn main() {}
