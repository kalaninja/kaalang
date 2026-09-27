use kaalang::kaalang;

#[kaalang]
fn prepared_partial_merge(mode: u8) -> String {
    #[choice("Prepare an owner or stay here.")]
    #[case("Prepare the first owner.")]
    #[case("Prepare the second owner.")]
    #[case("Stay here.")]
    let (first, second, stay) = |mode| match mode {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Prepare the first owner and entry.")]
    let (shared, go) = |first| (String::from("first"), ());

    #[action("Prepare the second owner and entry.")]
    let (shared, go) = |second| (String::from("second"), ());

    #[cycle("Stay on the remaining branch.")]
    |stay| {
        continue;
    };

    #[stage("Return the merged owner.")]
    |go| {
        |shared| return shared;
    };
}

#[test]
fn a_preparation_merge_survives_a_diverging_sibling() {
    assert_eq!(prepared_partial_merge(0), "first");
    assert_eq!(prepared_partial_merge(1), "second");
}
