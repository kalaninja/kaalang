use kaalang::kaalang;

#[kaalang("A description owned by an entry.")]
#[derive(Clone, Debug, PartialEq)]
struct Details {
    #[value("The description text.")]
    text: String,
}

#[kaalang("The description for the current revision.")]
type CurrentDetails = Details;

#[kaalang("The description retained for an earlier revision.")]
type RevisionDetails = Details;

#[kaalang("An ordered sequence of earlier descriptions.")]
#[refer(RevisionDetails, "Earlier descriptions in revision order.")]
type History = Vec<RevisionDetails>;

#[kaalang("An article with its current and earlier descriptions.")]
struct Article {
    #[refer(CurrentDetails, "The current description.")]
    current: CurrentDetails,
    #[refer(History, "The article's revision history.")]
    history: History,
    #[refer(RevisionDetails, "The revision readers saw last.")]
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
