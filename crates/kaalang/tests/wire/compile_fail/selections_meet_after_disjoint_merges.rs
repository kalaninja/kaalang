use kaalang::kaa;

// Each partial merge joins two cases, but no merge joins all four. The common
// question therefore opens an independent selection inside the choice's groups.
#[kaa]
fn invalid(mode: u8, report: bool) -> u8 {
    #[kaa::choice("Which source?")]
    #[kaa::case("First.")]
    #[kaa::case("Second.")]
    #[kaa::case("Third.")]
    #[kaa::case("Fourth.")]
    let (a, b, c, d) = |mode| {
        match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[kaa::action("First value.")]
    let early = |a| { 1 };

    #[kaa::action("Second value.")]
    let early = |b| { 2 };

    #[kaa::action("Third value.")]
    let late = |c| { 3 };

    #[kaa::action("Fourth value.")]
    let late = |d| { 4 };

    #[kaa::question("Report?")]
    let (yes, no) = |report| { report };

    #[kaa::action("Keep the yes marker.")]
    let kept = |yes| { 10 };

    #[kaa::action("Keep the no marker.")]
    let skipped = |no| { 20 };

    #[kaa::action("Finish the early half loudly.")]
    let end = |early, kept| { early + kept };

    #[kaa::action("Finish the early half quietly.")]
    let end = |early, skipped| { early + skipped };

    #[kaa::action("Finish the late half loudly.")]
    let end = |late, kept| { late + kept };

    #[kaa::action("Finish the late half quietly.")]
    let end = |late, skipped| { late + skipped };

    |end| return end;
}

fn main() {}
