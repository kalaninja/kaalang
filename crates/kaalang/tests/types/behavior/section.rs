use kaalang::kaalang;

#[kaalang("A key naming one section.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionId(u32);

// Declared before `Section` and still refers to it.
#[kaalang("A reader's remark pinned to a section.")]
struct Note {
    #[value("The remark.")]
    text: String,
    #[refer(Section, "The section the remark is pinned to.")]
    anchor: SectionId,
}

#[kaalang("A section of an article, nested to any depth.")]
struct Section {
    #[value("The section's key.")]
    id: SectionId,
    #[value("The section heading.")]
    heading: String,
    #[refer(Note, "Remarks pinned to this section.")]
    notes: Vec<Note>,
    #[refer(Section, "Subsections in reading order.")]
    children: Vec<Section>,
}

#[kaalang("An article's outline.")]
struct Outline {
    #[refer(Section, "Top-level sections in reading order.")]
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
