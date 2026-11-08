use kaalang::kaalang;

#[kaalang("A report.")]
struct Report {
    #[refer(Member, "The author.")]
    author: String,
}

fn main() {
    let _ = Report {
        author: String::new(),
    }
    .author;
}
