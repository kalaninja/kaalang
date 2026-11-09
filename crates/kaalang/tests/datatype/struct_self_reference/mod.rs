use kaalang::kaa;

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

#[kaa("A key naming one section.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionId(u32);

// Declared after `Section`, which refers to it.
#[kaa("A reader's remark pinned to a section.")]
struct Note {
    #[kaa::value("The remark.")]
    text: String,
    #[kaa::refer(Section, "The section the remark is pinned to.")]
    anchor: SectionId,
}

#[kaa("An article's outline.")]
struct Outline {
    #[kaa::refer(Section, "Top-level sections in reading order.")]
    sections: Vec<Section>,
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
