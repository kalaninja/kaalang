use kaalang::kaa;

#[kaa("A member.")]
struct Member;

#[kaa("A report.")]
struct Report {
    #[kaa::value("The author.")]
    #[kaa::refer(Member, "The author.")]
    author: Member,
}

fn main() {}
