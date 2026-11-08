use kaalang::kaalang;

#[kaalang("A member.")]
struct Member;

#[kaalang("A report.")]
struct Report {
    #[value("The author.")]
    #[refer(Member, "The author.")]
    author: Member,
}

fn main() {}
