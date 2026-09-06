use kaalang::kaalang;

// The left marker merges over the first two cases and the right marker over
// the last two, so neither merge group contains the other.
#[kaalang]
fn invalid(value: u8) -> u8 {
    #[choice("Which cases share which marker?")]
    #[case("Shares the left marker.")]
    #[case("Shares both markers.")]
    #[case("Shares the right marker.")]
    |value| -> (a, b, c) {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Mark the left group.")]
    |a| -> (_left, result) { ((), 1u8) };

    #[action("Mark both groups.")]
    |b| -> (_left, _right, result) { ((), (), 2u8) };

    #[action("Mark the right group.")]
    |c| -> (_right, result) { ((), 3u8) };
}

fn main() {}
