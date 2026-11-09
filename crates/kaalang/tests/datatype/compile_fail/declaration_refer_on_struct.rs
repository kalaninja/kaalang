use kaalang::kaa;

#[kaa("A member.")]
struct Member;

#[kaa("A key in the member directory.")]
#[kaa::refer(Member, "The member the key names.")]
struct MemberId(u32);

fn main() {}
