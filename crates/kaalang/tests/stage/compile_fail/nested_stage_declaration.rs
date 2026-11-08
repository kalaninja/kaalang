use kaalang::kaa;

#[kaa]
fn nested_stage_declaration(go: ()) {
    #[kaa::stage("Outer.")]
    |go| {
        #[kaa::stage("Inner.")]
        |go| {
            return;
        };
        return;
    };
}

fn main() {}
