#![allow(dead_code)]

use kaalang::kaalang;

#[kaalang("A key in the member directory.")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MemberId(u32);

#[kaalang("A member that can appear in several roles.")]
struct Member {
    #[value("The member's display name.")]
    name: String,
    active: bool,
}

#[kaalang("A member that may be unknown.")]
#[refer(Member, "The member, when known.")]
type MemberRef<'a> = Option<&'a Member>;

#[kaalang("A report with its assigned participants.")]
struct Report<'a> {
    #[value("The current revision.")]
    revision: u32,
    #[refer(Member, "The author.")]
    author: &'a Member,
    #[refer(Member, "The assignee, resolved through the directory.")]
    assignee: MemberId,
    #[refer(Member, "An optional delegate, resolved through the directory.")]
    delegate: Option<MemberId>,
    #[refer(Member, "Contributors in the order of their keys.")]
    contributors: Vec<MemberId>,
}

#[kaalang("Who may approve a change.")]
enum Approval<'a> {
    #[case("Anyone may approve.")]
    Open,
    Named(#[refer(Member, "The only approver.")] &'a Member),
    Quorum {
        #[value("The votes needed.")]
        votes: u8,
        #[refer(Member, "The members who may vote.")]
        voters: Vec<MemberId>,
    },
}

#[kaalang("A member key or its raw number, in shared storage.")]
union Lookup {
    #[refer(Member, "The member the key names.")]
    id: MemberId,
    #[value("The key as a number.")]
    raw: u32,
}

mod directory {
    use super::*;

    #[kaalang("A team led by one member.")]
    pub(super) struct Team {
        #[refer(Member, "The team lead.")]
        pub(super) lead: MemberId,
        #[refer(super::MemberId, "The keys of the other members.")]
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
