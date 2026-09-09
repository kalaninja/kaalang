use kaalang::kaalang;

// Each partial merge joins two cases, but no merge joins all four. The common
// question therefore opens an independent selection inside the choice's groups.
#[kaalang]
fn invalid(mode: u8, report: bool) -> u8 {
    #[choice("Which source?")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    #[case("Fourth.")]
    let (a, b, c, d) = |mode| {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("First value.")]
    let early = |a| { 1u8 };

    #[action("Second value.")]
    let early = |b| { 2u8 };

    #[action("Third value.")]
    let late = |c| { 3u8 };

    #[action("Fourth value.")]
    let late = |d| { 4u8 };

    #[question("Report?")]
    let (yes, no) = |report| { report };

    #[action("Keep the yes marker.")]
    let kept = |yes| { 10u8 };

    #[action("Keep the no marker.")]
    let skipped = |no| { 20u8 };

    #[action("Finish the early half loudly.")]
    let result = |early, kept| { early + kept };

    #[action("Finish the early half quietly.")]
    let result = |early, skipped| { early + skipped };

    #[action("Finish the late half loudly.")]
    let result = |late, kept| { late + kept };

    #[action("Finish the late half quietly.")]
    let result = |late, skipped| { late + skipped };
}

fn main() {}
