use kaalang::kaa;

#[kaa("A key naming one section.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionId(u32);

// Declared before `Section` and still refers to it.
#[kaa("A reader's remark pinned to a section.")]
struct Note {
    #[kaa::value("The remark.")]
    text: String,
    #[kaa::refer(Section, "The section the remark is pinned to.")]
    anchor: SectionId,
}

#[kaa("A section of an article, nested to any depth.")]
struct Section {
    #[kaa::value("The section's key.")]
    id: SectionId,
    #[kaa::value("The section heading.")]
    heading: String,
    #[kaa::refer(Note, "Remarks pinned to this section.")]
    notes: Vec<Note>,
    #[kaa::refer(Section, "Subsections in reading order.")]
    children: Vec<Section>,
}

#[kaa("An article's outline.")]
struct Outline {
    #[kaa::refer(Section, "Top-level sections in reading order.")]
    sections: Vec<Section>,
}

#[kaa("A query that selects sections.")]
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

#[kaa("A pool slot, free or holding a value, in shared storage.")]
union Slot {
    #[kaa::refer(Slot, "The next free slot, while this one is free.")]
    next: u32,
    #[kaa::value("The stored value, while this slot is taken.")]
    value: f32,
}

fn depth(section: &Section) -> usize {
    1 + section.children.iter().map(depth).max().unwrap_or(0)
}

#[test]
fn a_type_may_refer_to_itself() {
    let leaf = Section {
        id: SectionId(2),
        heading: String::from("Details"),
        notes: vec![Note {
            text: String::from("Expand this."),
            anchor: SectionId(2),
        }],
        children: Vec::new(),
    };
    let outline = Outline {
        sections: vec![Section {
            id: SectionId(1),
            heading: String::from("Overview"),
            notes: Vec::new(),
            children: vec![leaf],
        }],
    };

    let overview = &outline.sections[0];
    let note = &overview.children[0].notes[0];
    assert_eq!(depth(overview), 2);
    assert_eq!(note.anchor, overview.children[0].id);
    assert_eq!(
        (overview.heading.as_str(), note.text.as_str()),
        ("Overview", "Expand this.")
    );
}

fn selects(query: &Query, section: &Section) -> bool {
    match query {
        Query::Both(first, second) => selects(first, section) && selects(second, section),
        Query::Heading(text) => section.heading.contains(text.as_str()),
        Query::Not(query) => !selects(query, section),
    }
}

#[test]
fn an_enum_may_refer_to_itself_from_any_variant() {
    let section = Section {
        id: SectionId(1),
        heading: String::from("Overview"),
        notes: Vec::new(),
        children: Vec::new(),
    };
    let query = Query::Both(
        Box::new(Query::Heading(String::from("Over"))),
        Box::new(Query::Not(Box::new(Query::Heading(String::from(
            "Details",
        ))))),
    );

    assert!(selects(&query, &section));
}

#[test]
fn a_union_may_refer_to_itself() {
    let slots = [Slot { next: 1 }, Slot { value: 0.5 }];
    assert_eq!(size_of_val(&slots), 2 * size_of::<u32>());
}
