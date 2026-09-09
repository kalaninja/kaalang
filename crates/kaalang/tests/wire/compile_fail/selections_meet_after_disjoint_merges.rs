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
    |mode| -> (a, b, c, d) {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("First value.")]
    |a| -> early { 1u8 };

    #[action("Second value.")]
    |b| -> early { 2u8 };

    #[action("Third value.")]
    |c| -> late { 3u8 };

    #[action("Fourth value.")]
    |d| -> late { 4u8 };

    #[question("Report?")]
    |report| -> (yes, no) { report };

    #[action("Keep the yes marker.")]
    |yes| -> kept { 10u8 };

    #[action("Keep the no marker.")]
    |no| -> skipped { 20u8 };

    #[action("Finish the early half loudly.")]
    |early, kept| -> result { early + kept };

    #[action("Finish the early half quietly.")]
    |early, skipped| -> result { early + skipped };

    #[action("Finish the late half loudly.")]
    |late, kept| -> result { late + kept };

    #[action("Finish the late half quietly.")]
    |late, skipped| -> result { late + skipped };
}

fn main() {}
