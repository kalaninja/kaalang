use kaalang::kaa;

#[kaa("A section of an article.")]
struct Section;

#[kaa("An article's sections in reading order.")]
#[kaa::refer(Self, "The outline's sections.")]
type Outline = Vec<Section>;

fn main() {}
