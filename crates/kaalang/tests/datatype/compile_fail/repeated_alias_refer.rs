use kaalang::kaa;

#[kaa("A member.")]
struct Member;

#[kaa("Two members.")]
#[kaa::refer(Member, "The first member.")]
#[kaa::refer(Member, "The second member.")]
type Pair = (Member, Member);

fn main() {}
