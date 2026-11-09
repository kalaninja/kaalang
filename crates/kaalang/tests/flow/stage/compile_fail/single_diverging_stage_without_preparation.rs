use kaalang::kaa;

#[kaa]
fn single_diverging_stage_without_preparation(go: ()) -> ! {
    #[kaa::stage("Repeat forever.")]
    |go| {
        #[kaa::cycle("Stay in this stage.")]
        |go| loop {
            continue;
        };
    };
}

fn main() {}
