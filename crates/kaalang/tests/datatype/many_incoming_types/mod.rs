#![allow(dead_code)]

use kaalang::kaa;

#[kaa("A person with an account on the forum.")]
struct User {
    #[kaa::value("The user's key.")]
    id: UserId,
    #[kaa::value("The name shown on posts.")]
    name: String,
}

#[kaa("A key of a user account.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UserId(u32);

#[kaa("A signed-in browser.")]
struct Session {
    #[kaa::refer(User, "The signed-in user.")]
    user: UserId,
    #[kaa::value("When the session ends, in seconds.")]
    expires: u64,
}

#[kaa("A post in a thread.")]
struct Post {
    #[kaa::refer(User, "The author.")]
    author: UserId,
    #[kaa::value("The post text.")]
    text: String,
}

#[kaa("A reply to a post.")]
struct Comment {
    #[kaa::refer(User, "The author.")]
    author: UserId,
    #[kaa::value("The reply text.")]
    text: String,
}

#[kaa("One user following another.")]
struct Follow {
    #[kaa::refer(User, "Who follows.")]
    follower: UserId,
    #[kaa::refer(User, "Who is followed.")]
    followed: UserId,
}

#[kaa("A private message.")]
struct Message {
    #[kaa::refer(User, "The sender.")]
    sender: UserId,
    #[kaa::refer(User, "The recipient.")]
    recipient: UserId,
    #[kaa::value("The message text.")]
    text: String,
}

#[kaa("A group of users.")]
struct Group {
    #[kaa::refer(User, "The owner.")]
    owner: UserId,
    #[kaa::refer(User, "The members, owner included.")]
    members: Vec<UserId>,
}

#[kaa("An invitation to a group.")]
struct Invitation {
    #[kaa::refer(User, "Who invites.")]
    inviter: UserId,
    #[kaa::refer(User, "Who is invited.")]
    invitee: UserId,
}

#[kaa("A notice shown to one user.")]
struct Notification {
    #[kaa::refer(User, "Who sees it.")]
    recipient: UserId,
    #[kaa::value("What happened.")]
    text: String,
}

#[kaa("A complaint about content.")]
struct Complaint {
    #[kaa::refer(User, "Who complains.")]
    reporter: UserId,
    #[kaa::value("Why.")]
    reason: String,
}

#[kaa("Whether a user may post.")]
enum Standing {
    #[kaa::case("The user may post.")]
    Good,
    #[kaa::case("A moderator stopped the user from posting.")]
    Banned {
        #[kaa::refer(User, "The moderator.")]
        by: UserId,
        #[kaa::value("Until when, in seconds.")]
        until: u64,
    },
}

#[kaa("One change recorded for review.")]
struct AuditEntry {
    #[kaa::refer(User, "Who made the change.")]
    actor: UserId,
    #[kaa::value("What changed.")]
    change: String,
}

#[kaa("A user's subscription to a thread.")]
struct Subscription {
    #[kaa::refer(User, "The subscriber.")]
    subscriber: UserId,
    #[kaa::value("The thread's number.")]
    thread: u32,
}

#[kaa("An unpublished post.")]
struct Draft {
    #[kaa::refer(User, "The author.")]
    author: UserId,
    #[kaa::value("The text so far.")]
    text: String,
}

#[kaa("An emoji left on a post.")]
struct Reaction {
    #[kaa::refer(User, "Who reacted.")]
    user: UserId,
    #[kaa::value("The emoji.")]
    emoji: char,
}

#[kaa("A user named in a post.")]
struct Mention {
    #[kaa::refer(User, "Who is named.")]
    mentioned: UserId,
    #[kaa::value("Where the name starts in the text.")]
    offset: usize,
}

#[kaa("A user's moderator rights.")]
#[kaa::refer(User, "The moderator.")]
type Moderator = UserId;

#[test]
fn a_type_may_have_many_incoming_types() {
    let (sam, kim) = (UserId(1), UserId(2));
    let follow = Follow {
        follower: sam,
        followed: kim,
    };
    let message = Message {
        sender: kim,
        recipient: sam,
        text: String::from("Hello."),
    };
    let moderator: Moderator = kim;

    assert_eq!(follow.followed, message.sender);
    assert!(matches!(
        Standing::Banned { by: moderator, until: 60 },
        Standing::Banned { by, .. } if by == kim
    ));
}
