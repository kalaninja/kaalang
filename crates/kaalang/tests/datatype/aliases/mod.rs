#![allow(dead_code)]

use kaalang::kaa;

#[kaa("The description retained for an earlier revision.")]
type RevisionDetails = Details;

#[kaa("A description owned by an entry.")]
#[derive(Clone, Debug, PartialEq)]
struct Details {
    #[kaa::value("The description text.")]
    text: String,
}

#[kaa("The description for the current revision.")]
type CurrentDetails = Details;

#[kaa("An ordered sequence of earlier descriptions.")]
#[kaa::refer(RevisionDetails, "Earlier descriptions in revision order.")]
type History = Vec<RevisionDetails>;

#[kaa("An article with its current and earlier descriptions.")]
struct Article {
    #[kaa::refer(CurrentDetails, "The current description.")]
    current: CurrentDetails,
    #[kaa::refer(History, "The article's revision history.")]
    history: History,
    #[kaa::refer(RevisionDetails, "The revision readers saw last.")]
    last_seen: Option<RevisionDetails>,
}

#[test]
fn aliases_name_the_same_rust_type() {
    let details = Details {
        text: String::from("First draft."),
    };
    let article = Article {
        current: CurrentDetails::clone(&details),
        history: vec![details],
        last_seen: None,
    };

    assert_eq!(article.history[0], article.current);
    assert!(article.last_seen.is_none());
}
