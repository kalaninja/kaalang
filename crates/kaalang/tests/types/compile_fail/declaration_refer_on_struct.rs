use kaalang::kaalang;

#[kaalang("A member.")]
struct Member;

#[kaalang("A key in the member directory.")]
#[refer(Member, "The member the key names.")]
struct MemberId(u32);

fn main() {}
