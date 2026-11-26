use kaalang::kaa;

struct Target;

#[kaa]
struct Handle<#[cfg(all())] r#Target> {
    #[kaa::refer(Target, "The generic parameter.")]
    value: Target,
}

fn main() {}
