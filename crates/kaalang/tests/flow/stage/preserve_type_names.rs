use kaalang::kaa;

#[allow(non_camel_case_types)]
enum __kaalang_stage {
    Authored,
}

macro_rules! authored_variant {
    () => {
        matches!(__kaalang_stage::Authored, __kaalang_stage::Authored)
    };
}

#[kaa]
fn preserve_type_names(go: ()) -> bool {
    #[kaa::stage("Use the authored type.")]
    let finish = |go| {
        #[kaa::action("Resolve type names inside a macro.")]
        let finish = |go| authored_variant!();
    };

    #[kaa::stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn generated_dispatch_does_not_shadow_authored_types() {
    assert!(preserve_type_names(()));
}
