use kaalang::kaalang;

#[kaalang]
fn independent_effects(first: &mut Vec<&'static str>, second: &mut Vec<&'static str>) {
    #[action("Record the first effect.")]
    |first| -> first_done { first.push("first") };

    #[action("Record the second effect.")]
    |second| -> second_done { second.push("second") };

    #[action("Finish once both effects have run.")]
    |first_done, second_done| -> result {};
}

#[test]
fn independent_effects_each_run_once_in_the_one_execution() {
    let (mut first, mut second) = (Vec::new(), Vec::new());
    independent_effects(&mut first, &mut second);
    assert_eq!(first, ["first"]);
    assert_eq!(second, ["second"]);
}
