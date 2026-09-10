use kaalang::kaalang;

// The left marker merges over the first two cases and the right marker over
// the last two, so neither merge group contains the other.
#[kaalang]
fn invalid(value: u8) -> u8 {
    #[choice("Which cases share which marker?")]
    #[case("Shares the left marker.")]
    #[case("Shares both markers.")]
    #[case("Shares the right marker.")]
    let (a, b, c) = |value| {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Mark the left group.")]
    let (_left, end) = |a| { ((), 1u8) };

    #[action("Mark both groups.")]
    let (_left, _right, end) = |b| { ((), (), 2u8) };

    #[action("Mark the right group.")]
    let (_right, end) = |c| { ((), 3u8) };
}

fn main() {}
