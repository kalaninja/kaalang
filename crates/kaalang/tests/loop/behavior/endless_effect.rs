use kaalang::kaalang;

#[allow(dead_code)]
#[kaalang]
fn endless_effect(mut count: usize) -> ! {
    loop {
        #[action("Increment the counter.")]
        |&mut count| *count = count.wrapping_add(1);
    }
}
