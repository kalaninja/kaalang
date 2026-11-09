use kaalang::kaa;

#[kaa("A query that selects sections by their headings.")]
enum Query {
    #[kaa::case("Sections both queries select.")]
    Both(
        #[kaa::refer(Query, "The first query.")] Box<Query>,
        #[kaa::refer(Query, "The second query.")] Box<Query>,
    ),
    #[kaa::case("Sections whose heading contains the text.")]
    Heading(#[kaa::value("The text to find.")] String),
    #[kaa::case("Sections the query does not select.")]
    Not(#[kaa::refer(Query, "The query to invert.")] Box<Query>),
}

fn selects(query: &Query, heading: &str) -> bool {
    match query {
        Query::Both(first, second) => selects(first, heading) && selects(second, heading),
        Query::Heading(text) => heading.contains(text.as_str()),
        Query::Not(query) => !selects(query, heading),
    }
}

#[test]
fn an_enum_may_refer_to_itself_from_any_variant() {
    let query = Query::Both(
        Box::new(Query::Heading(String::from("Over"))),
        Box::new(Query::Not(Box::new(Query::Heading(String::from(
            "Details",
        ))))),
    );

    assert!(selects(&query, "Overview"));
    assert!(!selects(&query, "Overview Details"));
}
