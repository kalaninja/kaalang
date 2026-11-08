use kaalang::kaa;

#[kaa]
fn repeated_move_of_outer_state(data: String, go: ()) -> ! {
    #[kaa::stage("Consume and repeat.")]
    let go = |go| {
        #[kaa::action("Consume the shared value.")]
        |data| {
            drop(data);
        };
        #[kaa::action("Repeat the stage.")]
        let go = |go| ();
    };
}

fn main() {}
