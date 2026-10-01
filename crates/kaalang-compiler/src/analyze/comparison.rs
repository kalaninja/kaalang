//! Connects executions that disagree at exactly one shared selector.
//! See RFC 0007 §5.1 for the comparison reduction and its remaining bounds.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::model::Execution;

use super::only_difference;

/// At most 2016 pairs, cheaper than setting up the index.
pub(super) const PAIRWISE_EXECUTIONS: usize = 64;

/// For each context, visits enough compatible pairs to preserve connectivity.
/// `None` excludes an execution from that context. The callback receives its
/// context index, the differing selector and the two execution indices.
pub(super) fn compare<T>(
    executions: &[&Execution],
    context: &[Vec<Option<T>>],
    compare: impl FnMut(usize, usize, usize, usize),
) {
    if executions.len() <= PAIRWISE_EXECUTIONS {
        pairwise(executions, context, compare);
    } else {
        compatible(executions, context, compare);
    }
}

/// Compares every pair of executions. Bounded by [`PAIRWISE_EXECUTIONS`], this
/// stays under 2016 comparisons and needs nothing built up front.
pub(super) fn pairwise<T>(
    executions: &[&Execution],
    context: &[Vec<Option<T>>],
    mut compare: impl FnMut(usize, usize, usize, usize),
) {
    for (position, first) in executions.iter().enumerate() {
        for (offset, second) in executions[position + 1..].iter().enumerate() {
            let Some(selector) = only_difference(first, second) else {
                continue;
            };
            let other = position + 1 + offset;
            for (column, context) in context.iter().enumerate() {
                let (Some(_), Some(_)) = (&context[position], &context[other]) else {
                    continue;
                };
                compare(column, selector, position, other);
            }
        }
    }
}

/// Preserves each selector's compatible components with representative pairs.
/// Context filtering precedes representative selection; see RFC 0007 §5.1.
// ponytail: explicit histories with distinct selector sets still pair quadratically;
// larger production flows use execution conditions instead of this reference.
pub(super) fn compatible<T>(
    executions: &[&Execution],
    context: &[Vec<Option<T>>],
    mut compare: impl FnMut(usize, usize, usize, usize),
) {
    let shapes = shapes(executions);
    let mut common = Vec::new();
    let mut members = Vec::new();
    let mut projection = Vec::new();
    let mut order = Vec::new();
    let mut sides = Sides::default();
    for (position, (selectors, group)) in shapes.iter().enumerate() {
        for (offset, (others, other)) in shapes[position..].iter().enumerate() {
            let crossing = offset > 0;
            let (left, right) = if crossing {
                (group.as_slice(), other.as_slice())
            } else {
                (group.as_slice(), &group[..0])
            };
            let pairs = if crossing {
                left.len() * right.len()
            } else {
                left.len() * (left.len() - 1) / 2
            };
            if pairs == 0 {
                continue;
            }
            shared(selectors, others, &mut common);
            if common.is_empty() {
                continue;
            }
            // Sorting a run once per shared selector, on a comparison that
            // walks the others, only pays off when a group pair holds many
            // more members than the selectors they share. Below that its own
            // pairs cost less, and taking them is never worse than scanning
            // every pair of the flow: the group pairs partition those.
            let width = common.len();
            let reach = left.len() + right.len();
            if width * reach * reach.ilog2().max(1) as usize >= pairs {
                for (index, &first) in left.iter().enumerate() {
                    let rest = if crossing { right } else { &left[index + 1..] };
                    for &second in rest {
                        let Some(selector) = only_difference(executions[first], executions[second])
                        else {
                            continue;
                        };
                        for (column, context) in context.iter().enumerate() {
                            let (Some(_), Some(_)) = (&context[first], &context[second]) else {
                                continue;
                            };
                            compare(column, selector, first, second);
                        }
                    }
                }
                continue;
            }
            members.clear();
            members.extend(group);
            if crossing {
                members.extend(other);
            }
            projection.clear();
            for &member in &members {
                project(executions[member], &common, &mut projection);
            }
            for (column, &selector) in common.iter().enumerate() {
                let compatible = Compatible {
                    members: &members,
                    projection: &projection,
                    width,
                    column,
                    split: group.len(),
                    crossing,
                };
                order.clear();
                order.extend(0..members.len());
                order.sort_unstable_by(|&first, &second| compatible.compare(first, second));
                let mut start = 0;
                while start < order.len() {
                    let mut finish = start + 1;
                    while finish < order.len()
                        && compatible.compare(order[start], order[finish]).is_eq()
                    {
                        finish += 1;
                    }
                    for (column, context) in context.iter().enumerate() {
                        compatible.record(
                            &order[start..finish],
                            context,
                            &mut sides,
                            |first, second| {
                                compare(column, selector, first, second);
                            },
                        );
                    }
                    start = finish;
                }
            }
        }
    }
}

/// Executions grouped by the questions and choices they run, in block order.
/// Two executions can differ at one selector only when both run it, so every
/// pair worth comparing lies inside one pair of these groups.
fn shapes(executions: &[&Execution]) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut shapes = BTreeMap::<Vec<usize>, Vec<usize>>::new();
    for (index, execution) in executions.iter().enumerate() {
        shapes
            .entry(
                execution
                    .branches
                    .iter()
                    .map(|selection| selection.block)
                    .collect(),
            )
            .or_default()
            .push(index);
    }
    shapes.into_iter().collect()
}

/// The selectors both groups run. Both lists are sorted, so one walk finds them.
/// The buffer is reused across group pairs, of which a flow has many.
fn shared(first: &[usize], second: &[usize], shared: &mut Vec<usize>) {
    shared.clear();
    let (mut first, mut second) = (first, second);
    while let ([left, rest @ ..], [right, others @ ..]) = (first, second) {
        match left.cmp(right) {
            Ordering::Less => first = rest,
            Ordering::Greater => second = others,
            Ordering::Equal => {
                shared.push(*left);
                (first, second) = (rest, others);
            }
        }
    }
}

/// Appends the branches one execution takes at `shared`, in that order. Both
/// lists are sorted by block and `shared` only names selectors this execution
/// runs, so one walk finds every branch.
fn project(execution: &Execution, shared: &[usize], projection: &mut Vec<usize>) {
    let mut branches = execution.branches.iter();
    for &selector in shared {
        let selection = branches
            .find(|selection| selection.block == selector)
            .expect("a shared selector runs in both groups");
        projection.push(selection.branch);
    }
}

/// Scratch for one compatible group: its members on each side of the group
/// pair, kept between groups so the walk allocates nothing per group.
#[derive(Default)]
struct Sides {
    left: Vec<usize>,
    right: Vec<usize>,
}

/// One pair of selector-set groups, compared at one selector they share.
/// Members index `members`, which holds the left group and, when the pair
/// crosses two groups, the right one after `split`.
struct Compatible<'a> {
    members: &'a [usize],
    projection: &'a [usize],
    width: usize,
    column: usize,
    split: usize,
    crossing: bool,
}

impl Compatible<'_> {
    /// Orders members by every shared selector but the one under test. Equal
    /// members are the ones that may differ at it and nowhere else.
    fn compare(&self, first: usize, second: usize) -> Ordering {
        let (first, second) = (self.row(first), self.row(second));
        first[..self.column]
            .cmp(&second[..self.column])
            .then_with(|| first[self.column + 1..].cmp(&second[self.column + 1..]))
    }

    fn row(&self, member: usize) -> &[usize] {
        &self.projection[member * self.width..(member + 1) * self.width]
    }

    fn branch(&self, member: usize) -> usize {
        self.projection[member * self.width + self.column]
    }

    /// Hands one context the comparisons of one compatible group. Members of the
    /// group already agree everywhere else, so a differing branch at the
    /// selector is the whole test.
    fn record<T>(
        &self,
        group: &[usize],
        context: &[Option<T>],
        sides: &mut Sides,
        mut compare: impl FnMut(usize, usize),
    ) {
        sides.left.clear();
        sides.right.clear();
        for &member in group {
            if context[self.members[member]].is_none() {
                continue;
            }
            if member < self.split {
                sides.left.push(member);
            } else {
                sides.right.push(member);
            }
        }
        if self.crossing {
            for (members, others) in [(&sides.left, &sides.right), (&sides.right, &sides.left)] {
                let representatives = self.representatives(others);
                for &member in members {
                    for other in representatives.into_iter().flatten() {
                        if self.branch(member) != self.branch(other) {
                            compare(self.members[member], self.members[other]);
                        }
                    }
                }
            }
        } else {
            let [Some(first), Some(second)] = self.representatives(&sides.left) else {
                return;
            };
            for &member in &sides.left {
                let partner = if self.branch(member) == self.branch(first) {
                    second
                } else {
                    first
                };
                compare(self.members[member], self.members[partner]);
            }
        }
    }

    /// Up to two members taking different branches at the selector. Any third
    /// branch differs from one of them, which is what keeps every member of a
    /// compatible group connected to the rest through valid comparisons.
    fn representatives(&self, members: &[usize]) -> [Option<usize>; 2] {
        let mut representatives = [None, None];
        for &member in members {
            match representatives {
                [None, _] => representatives[0] = Some(member),
                [Some(first), None] if self.branch(first) != self.branch(member) => {
                    representatives[1] = Some(member);
                }
                _ => {}
            }
        }
        representatives
    }
}
