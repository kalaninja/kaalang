//! Data-capturing break statements use structural junctions, not nodes.

pub(super) fn project(count: &mut usize) -> usize {
    let index = *count;
    *count += 1;
    index
}
