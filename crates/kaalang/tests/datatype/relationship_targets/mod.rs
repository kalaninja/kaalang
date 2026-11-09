#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A key in the member directory.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MemberId(u32);

#[kaa("A member that can appear in several roles.")]
struct Member {
    #[kaa::value("The member's display name.")]
    name: String,
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

mod directory {
    use super::*;

    #[kaa("A team led by one member.")]
    pub(super) struct Team {
        #[kaa::refer(Member, "The team lead.")]
        pub(super) lead: MemberId,
        // A path target: these diagrams follow bare names only, standing in for
        // the cargo kaalang resolver.
        #[kaa::refer(super::MemberId, "The keys of the other members.")]
        pub(super) others: Vec<MemberId>,
    }
}

#[test]
fn relationships_name_their_target_apart_from_the_field_type() {
    let member = Member {
        name: String::from("Sam"),
    };
    let known: MemberRef<'_> = Some(&member);
    let report = Report {
        revision: 1,
        author: &member,
        assignee: MemberId(7),
        delegate: None,
        contributors: vec![MemberId(7)],
    };
    let team = directory::Team {
        lead: MemberId(7),
        others: Vec::new(),
    };

    assert_eq!(known.map(|member| member.name.as_str()), Some("Sam"));
    assert_eq!(report.assignee, team.lead);
}
