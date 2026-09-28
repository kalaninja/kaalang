use kaalang::kaalang;

#[kaalang]
fn nested_stage_declaration(go: ()) {
    #[stage("Outer.")]
    |go| {
        #[stage("Inner.")]
        |go| {
            return;
        };
        return;
    };
}

fn main() {}
