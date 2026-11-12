//! Four ways to combine three Boolean values.

mod and;
mod inverted_and;
mod inverted_or;
mod or;

pub(super) fn check() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(and::and(a, b, c), a && b && c);
                assert_eq!(or::or(a, b, c), a || b || c);
                assert_eq!(inverted_and::inverted_and(a, b, c), !(a && b && c));
                assert_eq!(inverted_or::inverted_or(a, b, c), !(a || b || c));
            }
        }
    }
}
