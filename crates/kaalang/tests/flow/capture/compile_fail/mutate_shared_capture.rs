use kaalang::kaa;

#[kaa]
fn mutate_shared_capture(text: String) -> usize {
    #[kaa::action("Try to mutate a shared reference.")]
    let end = |&text| {
        text.push('!');
        text.len()
    };

    |end| return end;
}

fn main() {}
