use kaalang::kaalang;

#[kaalang]
fn single_diverging_stage_without_preparation(go: ()) -> ! {
    #[stage("Repeat forever.")]
    |go| {
        #[cycle("Stay in this stage.")]
        |go| {
            continue;
        };
    };
}

fn main() {}
