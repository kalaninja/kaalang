#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A key in the member directory.")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MemberId(u32);

#[kaa("A member that can appear in several roles.")]
struct Member {
    #[kaa::value("The member's display name.")]
    name: String,
    active: bool,
}

#[kaa("A member that may be unknown.")]
#[kaa::refer(Member, "The member, when known.")]
type MemberRef<'a> = Option<&'a Member>;

#[kaa("A report with its assigned participants.")]
struct Report<'a> {
    #[kaa::value("The current revision.")]
    revision: u32,
    #[kaa::refer(Member, "The author.")]
    author: &'a Member,
    #[kaa::refer(Member, "The assignee, resolved through the directory.")]
    assignee: MemberId,
    #[kaa::refer(Member, "An optional delegate, resolved through the directory.")]
    delegate: Option<MemberId>,
    #[kaa::refer(Member, "Contributors in the order of their keys.")]
    contributors: Vec<MemberId>,
}

#[kaa("Who may approve a change.")]
enum Approval<'a> {
    #[kaa::case("Anyone may approve.")]
    Open,
    Named(#[kaa::refer(Member, "The only approver.")] &'a Member),
    Quorum {
        #[kaa::value("The votes needed.")]
        votes: u8,
        #[kaa::refer(Member, "The members who may vote.")]
        voters: Vec<MemberId>,
    },
}

#[kaa("A member key or its raw number, in shared storage.")]
union Lookup {
    #[kaa::refer(Member, "The member the key names.")]
    id: MemberId,
    #[kaa::value("The key as a number.")]
    raw: u32,
}

mod directory {
    use super::*;

    #[kaa("A team led by one member.")]
    pub(super) struct Team {
        #[kaa::refer(Member, "The team lead.")]
        pub(super) lead: MemberId,
        #[kaa::refer(super::MemberId, "The keys of the other members.")]
        pub(super) others: Vec<MemberId>,
    }
}

#[test]
fn annotated_types_stay_ordinary_rust_types() {
    let member = Member {
        name: String::from("Sam"),
        active: true,
    };
    let known: MemberRef<'_> = Some(&member);
    let report = Report {
        revision: 1,
        author: &member,
        assignee: MemberId(7),
        delegate: None,
        contributors: vec![MemberId(7)],
    };
    let approval = Approval::Named(&member);
    let team = directory::Team {
        lead: MemberId(7),
        others: Vec::new(),
    };

    assert_eq!(known.map(|member| member.name.as_str()), Some("Sam"));
    assert_eq!(report.assignee, team.lead);
    assert!(matches!(approval, Approval::Named(named) if std::ptr::eq(named, report.author)));
}

#[test]
fn a_union_keeps_its_shared_storage() {
    assert_eq!(size_of::<Lookup>(), size_of::<u32>());
}
