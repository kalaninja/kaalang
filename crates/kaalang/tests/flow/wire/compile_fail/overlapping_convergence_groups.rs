use kaalang::kaa;

// The left marker merges over the first two cases and the right marker over
// the last two, so neither merge group contains the other.
#[kaa]
fn invalid(value: u8) -> u8 {
    #[kaa::choice("Which cases share which marker?")]
    #[kaa::case("Shares the left marker.")]
    #[kaa::case("Shares both markers.")]
    #[kaa::case("Shares the right marker.")]
    let (a, b, c) = |value| {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[kaa::action("Mark the left group.")]
    let (_left, end) = |a| { ((), 1) };

    #[kaa::action("Mark both groups.")]
    let (_left, _right, end) = |b| { ((), (), 2) };

    #[kaa::action("Mark the right group.")]
    let (_right, end) = |c| { ((), 3) };

    |end| return end;
}

fn main() {}
