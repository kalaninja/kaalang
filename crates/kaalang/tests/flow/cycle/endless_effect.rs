use kaalang::kaa;

#[allow(dead_code)]
#[kaa]
fn endless_effect(mut count: usize) -> ! {
    #[kaa::cycle("Increment forever.")]
    loop {
        #[kaa::action("Increment the counter.")]
        |&mut count| *count = count.wrapping_add(1);

        continue;
    }
}
