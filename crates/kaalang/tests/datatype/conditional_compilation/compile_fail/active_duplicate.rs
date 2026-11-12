use kaalang::kaa;

#[kaa]
struct Record {
    #[cfg_attr(all(), kaa::value("The first description."))]
    #[cfg_attr(all(), kaa::value("The second description."))]
    value: u8,
}

fn main() {}
