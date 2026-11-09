use kaalang::kaa;

#[kaa]
fn transition_decided_twice(count: u8) -> u8 {
    #[kaa::action("Enter the stage.")]
    let go = || ();

    #[kaa::stage("Choose an exit.")]
    let (go, found, other) = |go| {
        #[kaa::question("Is the count even?")]
        let (even, odd) = |count| count % 2 == 0;

        #[kaa::question("Is it still even?")]
        let (again, even_found) = |even, count| count % 2 == 0;

        #[kaa::question("Is it a multiple of three?")]
        let (third, rest) = |odd, count| count % 3 == 0;

        #[kaa::action("Repeat the stage.")]
        let go = |again| ();

        #[kaa::action("Finish the even route.")]
        let found = |even_found| ();

        #[kaa::action("Finish the divisible route.")]
        let found = |third| ();

        #[kaa::action("Finish the remaining route.")]
        let other = |rest| ();
    };

    #[kaa::stage("Forward a found result.")]
    let end = |found| {
        #[kaa::action("Finish the found route.")]
        let end = |found| ();
    };

    #[kaa::stage("Forward the other result.")]
    let end = |other| {
        #[kaa::action("Finish the other route.")]
        let end = |other| ();
    };

    #[kaa::stage("Return.")]
    |end| {
        |count| return count;
    };
}

fn main() {}
