use kaalang::kaa;

#[kaa]
fn prepared_partial_merge(mode: u8) -> String {
    #[kaa::choice("Prepare an owner or stay here.")]
    #[kaa::case("Prepare the first owner.")]
    #[kaa::case("Prepare the second owner.")]
    #[kaa::case("Stay here.")]
    let (first, second, stay) = |mode| match mode {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Prepare the first owner and entry.")]
    let (shared, go) = |first| (String::from("first"), ());

    #[kaa::action("Prepare the second owner and entry.")]
    let (shared, go) = |second| (String::from("second"), ());

    #[kaa::cycle("Stay on the remaining branch.")]
    |stay| loop {
        continue;
    };

    #[kaa::stage("Return the merged owner.")]
    |go| {
        |shared| return shared;
    };
}

#[test]
fn a_preparation_merge_survives_a_diverging_sibling() {
    assert_eq!(prepared_partial_merge(0), "first");
    assert_eq!(prepared_partial_merge(1), "second");
}
