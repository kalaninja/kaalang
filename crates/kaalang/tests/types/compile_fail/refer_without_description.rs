use kaalang::kaalang;

#[kaalang("A member.")]
struct Member;

#[kaalang("A report.")]
struct Report {
    #[refer(Member)]
    author: Member,
}

fn main() {}
