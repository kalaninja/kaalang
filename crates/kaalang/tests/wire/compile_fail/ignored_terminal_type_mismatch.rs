use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes value and a marker.")]
    let (selected, _marker) = |yes| { (1, "yes") };

    #[action("Build the no result and a marker.")]
    let (result, _marker) = |no| { (2, 3u8) };

    #[action("Use the selected value.")]
    let result = |selected| { selected };
}

fn main() {}
