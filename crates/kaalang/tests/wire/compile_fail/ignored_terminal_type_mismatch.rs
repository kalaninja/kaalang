use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and a marker.")]
    |yes| -> (selected, _marker) { (1, "yes") };

    #[action("Build the no result and a marker.")]
    |no| -> (result, _marker) { (2, 3u8) };

    #[action("Use the selected value.")]
    |selected| -> result { selected };
}

fn main() {}
