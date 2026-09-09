use kaalang::kaalang;

struct Calculator;

impl Calculator {
    #[kaalang]
    fn method(&self, value: i32) -> i32 {
        #[action("Double the value.")]
        let doubled = |value| { value * 2 };
    }
}

fn main() {}
