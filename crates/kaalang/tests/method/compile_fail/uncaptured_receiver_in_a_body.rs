use kaalang::kaalang;

struct Counter {
    value: u32,
}

impl Counter {
    #[kaalang]
    fn invalid(&mut self, step: u32) -> u32 {
        #[action("Advance the counter.")]
        let advanced = |&mut self, step| {
            self.value += step;
        };

        #[action("Mutate the receiver without capturing it.")]
        let end = |advanced| {
            self.value *= 2;
            self.value
        };

        |end| return end;
    }
}

fn main() {}
