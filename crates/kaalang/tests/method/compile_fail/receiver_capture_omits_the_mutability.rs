use kaalang::kaalang;

struct Counter {
    value: u32,
}

impl Counter {
    #[kaalang]
    fn invalid(&mut self) -> u32 {
        #[action("Read the counter through a shared capture.")]
        let end = |&self| self.value;

        |end| return end;
    }
}

fn main() {}
