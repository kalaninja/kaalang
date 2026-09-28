use kaalang::kaalang;

#[kaalang]
fn moved_stage_signal_before_export(go: ()) -> String {
    #[stage("Move before export.")]
    let finish = |go| {
        #[action("Create text.")]
        let finish = |go| String::from("text");
        #[action("Move text.")]
        |finish| {
            drop(finish);
        };
    };
    #[stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
