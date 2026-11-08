use kaalang::kaa;

struct Counter {
    value: u32,
}

impl Counter {
    #[kaa]
    fn invalid(&mut self, step: u32) -> u32 {
        #[kaa::action("Advance the counter.")]
        let advanced = |&mut self, step| {
            self.value += step;
        };

        #[kaa::action("Mutate the receiver without capturing it.")]
        let end = |advanced| {
            self.value *= 2;
            self.value
        };

        |end| return end;
    }
}

fn main() {}
