use kaalang::kaa;

#[kaa("A member.")]
struct Member;

#[kaa("A report.")]
struct Report {
    #[kaa::refer(Member, "The author.")]
    #[kaa::refer(Member, "The reviewer.")]
    author: Member,
}

fn main() {}
