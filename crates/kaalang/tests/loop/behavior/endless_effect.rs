use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn endless_effect(mut count: usize) -> ! {
    #[cycle("Increment forever.")]
    {
        #[action("Increment the counter.")]
        |&mut count| *count = count.wrapping_add(1);

        continue;
    };
}
