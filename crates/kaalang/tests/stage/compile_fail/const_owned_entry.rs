use kaalang::kaalang;

#[kaalang]
const fn const_owned_entry(go: String) -> String {
    #[stage("Read the string.")]
    |go| {
        |go| return go;
    };
}

fn main() {}
