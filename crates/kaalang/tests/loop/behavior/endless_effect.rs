use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn endless_effect(count: usize) -> ! {
    #[cycle("Increment forever.")]
    |mut count| {
        #[action("Increment the counter.")]
        |&mut count| *count = count.wrapping_add(1);
    };
}
