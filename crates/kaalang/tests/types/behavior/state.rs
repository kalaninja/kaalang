use kaalang::kaa;

#[kaa]
#[repr(u8)]
#[derive(Debug, PartialEq)]
enum State {
    #[kaa::case("Not yet published.")]
    Draft,
    Published {
        #[kaa::value("The published revision.")]
        revision: u32,
        #[kaa::value("The publication note.")]
        note: String,
    },
    Archived(#[kaa::value("The archive reason.")] String),
}

#[kaa("A document with its publication state.")]
struct Document {
    #[kaa::value("The display title.")]
    title: String,
    #[kaa::refer(State, "The current publication state.")]
    state: State,
}

#[test]
fn a_bare_kaalang_type_keeps_its_variants() {
    let document = Document {
        title: String::from("Notes"),
        state: State::Published {
            revision: 2,
            note: String::from("Reviewed."),
        },
    };
    let archived = State::Archived(String::from("Superseded."));

    assert_eq!(document.title, "Notes");
    assert_ne!(document.state, State::Draft);
    assert_ne!(archived, document.state);
}
