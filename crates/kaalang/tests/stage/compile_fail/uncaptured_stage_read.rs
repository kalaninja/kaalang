use kaalang::kaalang;

#[kaalang]
fn uncaptured_stage_read(go: u8) -> u8 {
    #[stage("Read.")]
    let answer = |go| {
        #[action("Read without a capture.")]
        let answer = || go + 1;
    };

    #[stage("Return the result.")]
    |answer| {
        |answer| return answer;
    };
}

fn main() {}
