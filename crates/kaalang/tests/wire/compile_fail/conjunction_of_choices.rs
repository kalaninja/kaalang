use kaalang::kaalang;

#[kaalang]
fn invalid(left: bool, right: bool) {
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
    |a, b| -> () { assert_eq!(a + b, 11) };

    #[end]
    || {};
}

fn main() {}
