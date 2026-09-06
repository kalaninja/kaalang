use kaalang::kaalang;

// The payloads meet only when both choices select their first case, so the
// other executions leave the flow without its `result` wire.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[choice("Choose the left input.")]
    #[case("Provide a value.")]
    #[case("Leave the input absent.")]
    |left| -> (a, _left_no) {
        match left {
            true => 10,
            false => (),
        }
    };

    #[choice("Choose the right input.")]
    #[case("Provide a value.")]
    #[case("Leave the input absent.")]
    |right| -> (b, _right_no) {
        match right {
            true => 1,
            false => (),
        }
    };

    #[action("Require both selected payloads.")]
    |a, b| -> result { a + b };
}

fn main() {}
