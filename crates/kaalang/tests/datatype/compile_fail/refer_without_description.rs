use kaalang::kaa;

#[kaa("A member.")]
struct Member;

#[kaa("A report.")]
struct Report {
    #[kaa::refer(Member)]
    author: Member,
}

fn main() {}
