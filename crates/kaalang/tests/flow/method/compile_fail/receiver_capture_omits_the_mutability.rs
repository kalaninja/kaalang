use kaalang::kaa;

struct Counter {
    value: u32,
}

impl Counter {
    #[kaa]
    fn invalid(&mut self) -> u32 {
        #[kaa::action("Read the counter through a shared capture.")]
        let end = |&self| self.value;

        |end| return end;
    }
}

fn main() {}
