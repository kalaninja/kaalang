use kaalang::kaalang;

#[kaalang("A member.")]
struct Member;

#[kaalang("A report.")]
struct Report {
    #[refer(Member, "The author.")]
    #[refer(Member, "The reviewer.")]
    author: Member,
}

fn main() {}
