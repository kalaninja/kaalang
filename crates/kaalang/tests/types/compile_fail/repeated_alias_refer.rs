use kaalang::kaalang;

#[kaalang("A member.")]
struct Member;

#[kaalang("Two members.")]
#[refer(Member, "The first member.")]
#[refer(Member, "The second member.")]
type Pair = (Member, Member);

fn main() {}
