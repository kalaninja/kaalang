use kaalang::kaalang;

#[kaalang]
fn transition_decided_twice(count: u8) -> u8 {
    #[action("Enter the stage.")]
    let go = || ();

    #[stage("Choose an exit.")]
    let (go, found, other) = |go| {
        #[question("Is the count even?")]
        let (even, odd) = |count| count % 2 == 0;

        #[question("Is it still even?")]
        let (again, even_found) = |even, count| count % 2 == 0;

        #[question("Is it a multiple of three?")]
        let (third, rest) = |odd, count| count % 3 == 0;

        #[action("Repeat the stage.")]
        let go = |again| ();

        #[action("Finish the even route.")]
        let found = |even_found| ();

        #[action("Finish the divisible route.")]
        let found = |third| ();

        #[action("Finish the remaining route.")]
        let other = |rest| ();
    };

    #[stage("Forward a found result.")]
    let end = |found| {
        #[action("Finish the found route.")]
        let end = |found| ();
    };

    #[stage("Forward the other result.")]
    let end = |other| {
        #[action("Finish the other route.")]
        let end = |other| ();
    };

    #[stage("Return.")]
    |end| {
        |count| return count;
    };
}

fn main() {}
