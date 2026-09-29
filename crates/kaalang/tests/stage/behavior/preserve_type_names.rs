use kaalang::kaalang;

#[allow(non_camel_case_types)]
enum __kaalang_stage {
    Authored,
}

macro_rules! authored_variant {
    () => {
        matches!(__kaalang_stage::Authored, __kaalang_stage::Authored)
    };
}

#[kaalang]
fn preserve_type_names(go: ()) -> bool {
    #[stage("Use the authored type.")]
    let finish = |go| {
        #[action("Resolve type names inside a macro.")]
        let finish = |go| authored_variant!();
    };

    #[stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn generated_dispatch_does_not_shadow_authored_types() {
    assert!(preserve_type_names(()));
}
