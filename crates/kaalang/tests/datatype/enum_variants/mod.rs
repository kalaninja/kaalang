#![allow(dead_code)]

use kaalang::kaa;

#[kaa("Who may approve a change.")]
enum Approval<'a> {
    #[kaa::case("Anyone may approve.")]
    Open,
    Named(
        #[kaa::refer(Member, "The only approver.")] &'a Member,
        #[kaa::refer(Member, "A deputy who may stand in.")] Option<&'a Member>,
    ),
    #[kaa::case("A number of members must agree.")]
    Quorum {
        #[kaa::value("The votes needed.")]
        votes: u8,
        #[kaa::refer(Member, "The members who may vote.")]
        voters: Vec<&'a Member>,
    },
}

#[kaa("A member who may approve a change.")]
struct Member {
    #[kaa::value("The member's display name.")]
    name: String,
}

#[test]
fn enum_variants_keep_their_own_fields() {
    let sam = Member {
        name: String::from("Sam"),
    };
    let named = Approval::Named(&sam, None);
    let quorum = Approval::Quorum {
        votes: 2,
        voters: vec![&sam],
    };

    assert!(matches!(named, Approval::Named(member, None) if member.name == "Sam"));
    assert!(matches!(quorum, Approval::Quorum { votes: 2, voters } if voters.len() == 1));
}
