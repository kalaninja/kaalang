use kaalang::kaa;

#[kaa("A report.")]
struct Report {
    #[kaa::refer(Member, "The author.")]
    author: String,
}

fn main() {
    let _ = Report {
        author: String::new(),
    }
    .author;
}
