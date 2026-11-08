use kaalang::kaalang;

#[kaalang]
#[repr(u8)]
#[derive(Debug, PartialEq)]
enum State {
    #[case("Not yet published.")]
    Draft,
    Published {
        #[value("The published revision.")]
        revision: u32,
        #[value("The publication note.")]
        note: String,
    },
    Archived(#[value("The archive reason.")] String),
}

#[kaalang("A document with its publication state.")]
struct Document {
    #[value("The display title.")]
    title: String,
    #[refer(State, "The current publication state.")]
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
