use kaalang::kaa;

#[kaa]
fn moved_stage_signal_before_export(go: ()) -> String {
    #[kaa::stage("Move before export.")]
    let finish = |go| {
        #[kaa::action("Create text.")]
        let finish = |go| String::from("text");
        #[kaa::action("Move text.")]
        |finish| {
            drop(finish);
        };
    };
    #[kaa::stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
