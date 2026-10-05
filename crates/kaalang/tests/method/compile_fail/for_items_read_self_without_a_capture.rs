use kaalang::kaalang;

struct Values {
    values: Vec<u32>,
}

impl Values {
    #[kaalang]
    fn invalid(&self) {
        #[cycle("Visit every value.")]
        || for value in self.values.iter() {
            #[action("Use the value.")]
            |value| drop(value);
        };

        return;
    }
}

fn main() {}
