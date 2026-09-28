use kaalang::kaalang;

#[kaalang]
fn repeated_move_of_outer_state(data: String, go: ()) -> ! {
    #[stage("Consume and repeat.")]
    let go = |go| {
        #[action("Consume the shared value.")]
        |data| {
            drop(data);
        };
        #[action("Repeat the stage.")]
        let go = |go| ();
    };
}

fn main() {}
