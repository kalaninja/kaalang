//! Canonical conditions over structural selections, with shared suffixes.

use std::collections::HashMap;

pub(super) type Condition = usize;
pub(super) const NEVER: Condition = 0;
pub(super) const ALWAYS: Condition = 1;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Node {
    variable: usize,
    children: Vec<Condition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_match_complete_truth_tables_and_survive_compaction() {
        let mut conditions = Conditions::new(vec![3, 3, 4, 4, 2, 2]);
        let first = conditions.selected(0, 1);
        let second = conditions.selected(2, 2);
        let third = conditions.selected(4, 1);
        let either = conditions.or(first, second);
        let expression = conditions.minus(either, third);
        let partner = conditions.partner(expression);
        let projected = conditions.project(expression, &[0, 4]);
        let complement = conditions.not(expression);
        let restricted = conditions.restrict(expression, 0, 0);
        assert_eq!(conditions.count(expression), 6);
        assert_eq!(conditions.count_up_to(expression, Some(5)), 5);
        assert_eq!(conditions.count_up_to(expression, Some(7)), 6);
        let enormous = Conditions::new(vec![2; (usize::BITS as usize + 1) * 2]);
        assert_eq!(enormous.count_up_to(ALWAYS, Some(65)), 65);
        assert_eq!(enormous.count_up_to(NEVER, Some(65)), 0);
        let mut retained = conditions.clone();
        let mut roots = [expression, partner, projected, complement, restricted];
        retained.retain(&mut roots);
        for a in 0..3 {
            for b in 0..3 {
                for c in 0..4 {
                    for d in 0..4 {
                        for e in 0..2 {
                            for f in 0..2 {
                                let assignment = [a, b, c, d, e, f];
                                let expected = [
                                    (a == 1 || c == 2) && e != 1,
                                    (b == 1 || d == 2) && f != 1,
                                    e != 1,
                                    !((a == 1 || c == 2) && e != 1),
                                    c == 2 && e != 1,
                                ];
                                for (&condition, expected) in roots.iter().zip(expected) {
                                    assert_eq!(
                                        retained.matches(condition, &assignment),
                                        expected,
                                        "{assignment:?}"
                                    );
                                    if let Some(witness) = retained.witness(condition) {
                                        assert!(retained.matches(condition, &witness));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct Conditions {
    pub(super) widths: Vec<usize>,
    nodes: Vec<Node>,
    unique: HashMap<Node, Condition>,
    apply: HashMap<(bool, Condition, Condition), Condition>,
    complements: HashMap<Condition, Condition>,
    restrictions: HashMap<(Condition, usize, usize), Condition>,
}

impl Conditions {
    /// Retains only the predicates needed by the saved semantic model.
    pub(super) fn retain(&mut self, roots: &mut [Condition]) {
        fn copy(
            old: &[Node],
            new: &mut Conditions,
            id: Condition,
            copied: &mut HashMap<Condition, Condition>,
        ) -> Condition {
            if id < 2 {
                return id;
            }
            if let Some(&result) = copied.get(&id) {
                return result;
            }
            let node = &old[id - 2];
            let children = node
                .children
                .iter()
                .map(|&child| copy(old, new, child, copied))
                .collect();
            let result = new.node(node.variable, children);
            copied.insert(id, result);
            result
        }
        let mut retained = Self::new(self.widths.clone());
        let mut copied = HashMap::new();
        for root in roots {
            *root = copy(&self.nodes, &mut retained, *root, &mut copied);
        }
        *self = retained;
    }

    pub(super) fn new(widths: Vec<usize>) -> Self {
        Self {
            widths,
            nodes: Vec::new(),
            unique: HashMap::new(),
            apply: HashMap::new(),
            complements: HashMap::from([(NEVER, ALWAYS), (ALWAYS, NEVER)]),
            restrictions: HashMap::new(),
        }
    }

    fn node(&mut self, variable: usize, children: Vec<Condition>) -> Condition {
        debug_assert!(
            children
                .iter()
                .all(|&child| self.variable(child) > variable)
        );
        if children.iter().all(|&child| child == children[0]) {
            return children[0];
        }
        let node = Node { variable, children };
        if let Some(&existing) = self.unique.get(&node) {
            return existing;
        }
        let id = self.nodes.len() + 2;
        self.nodes.push(node.clone());
        self.unique.insert(node, id);
        id
    }

    pub(super) fn selected(&mut self, variable: usize, branch: usize) -> Condition {
        let mut children = vec![NEVER; self.widths[variable]];
        children[branch] = ALWAYS;
        self.node(variable, children)
    }

    pub(super) fn and(&mut self, first: Condition, second: Condition) -> Condition {
        self.combine(false, first, second)
    }

    pub(super) fn or(&mut self, first: Condition, second: Condition) -> Condition {
        self.combine(true, first, second)
    }

    pub(super) fn minus(&mut self, first: Condition, second: Condition) -> Condition {
        let other = self.not(second);
        self.and(first, other)
    }

    pub(super) fn xor(&mut self, first: Condition, second: Condition) -> Condition {
        let left = self.minus(first, second);
        let right = self.minus(second, first);
        self.or(left, right)
    }

    pub(super) fn not(&mut self, condition: Condition) -> Condition {
        if let Some(&complement) = self.complements.get(&condition) {
            return complement;
        }
        let node = self.nodes[condition - 2].clone();
        let children = node
            .children
            .into_iter()
            .map(|child| self.not(child))
            .collect();
        let complement = self.node(node.variable, children);
        self.complements.insert(condition, complement);
        self.complements.insert(complement, condition);
        complement
    }

    fn variable(&self, condition: Condition) -> usize {
        if condition < 2 {
            usize::MAX
        } else {
            self.nodes[condition - 2].variable
        }
    }

    fn combine(&mut self, union: bool, first: Condition, second: Condition) -> Condition {
        let (first, second) = (first.min(second), first.max(second));
        if first == second || (union && first == NEVER) || (!union && first == ALWAYS) {
            return second;
        }
        if (!union && first == NEVER) || (union && (first == ALWAYS || second == ALWAYS)) {
            return if union { ALWAYS } else { NEVER };
        }
        let key = (union, first, second);
        if let Some(&result) = self.apply.get(&key) {
            return result;
        }
        let variable = self.variable(first).min(self.variable(second));
        let children = (0..self.widths[variable])
            .map(|branch| {
                let child = |id| {
                    if self.variable(id) == variable {
                        self.nodes[id - 2].children[branch]
                    } else {
                        id
                    }
                };
                self.combine(union, child(first), child(second))
            })
            .collect();
        let result = self.node(variable, children);
        self.apply.insert(key, result);
        result
    }

    pub(super) fn restrict(
        &mut self,
        condition: Condition,
        variable: usize,
        value: usize,
    ) -> Condition {
        if self.variable(condition) > variable {
            return condition;
        }
        let key = (condition, variable, value);
        if let Some(&result) = self.restrictions.get(&key) {
            return result;
        }
        let node = self.nodes[condition - 2].clone();
        let result = if node.variable == variable {
            node.children[value]
        } else {
            let children = node
                .children
                .into_iter()
                .map(|child| self.restrict(child, variable, value))
                .collect();
            self.node(node.variable, children)
        };
        self.restrictions.insert(key, result);
        result
    }

    /// Projects away every selection not in `retained`.
    pub(super) fn project(&mut self, condition: Condition, retained: &[usize]) -> Condition {
        fn visit(
            c: &mut Conditions,
            id: Condition,
            retained: &[usize],
            cache: &mut HashMap<Condition, Condition>,
        ) -> Condition {
            if id < 2 {
                return id;
            }
            if let Some(&result) = cache.get(&id) {
                return result;
            }
            let node = c.nodes[id - 2].clone();
            let children: Vec<_> = node
                .children
                .into_iter()
                .map(|child| visit(c, child, retained, cache))
                .collect();
            let result = if retained.contains(&node.variable) {
                c.node(node.variable, children)
            } else {
                children
                    .into_iter()
                    .fold(NEVER, |sum, child| c.or(sum, child))
            };
            cache.insert(id, result);
            result
        }
        visit(self, condition, retained, &mut HashMap::new())
    }

    /// Moves a single-execution predicate to the adjacent variables of its partner.
    pub(super) fn partner(&mut self, condition: Condition) -> Condition {
        fn visit(
            c: &mut Conditions,
            id: Condition,
            cache: &mut HashMap<Condition, Condition>,
        ) -> Condition {
            if id < 2 {
                return id;
            }
            if let Some(&result) = cache.get(&id) {
                return result;
            }
            let node = c.nodes[id - 2].clone();
            debug_assert_eq!(node.variable % 2, 0);
            let children = node
                .children
                .into_iter()
                .map(|child| visit(c, child, cache))
                .collect();
            let result = c.node(node.variable + 1, children);
            cache.insert(id, result);
            result
        }
        visit(self, condition, &mut HashMap::new())
    }

    pub(super) fn matches(&self, mut condition: Condition, assignment: &[usize]) -> bool {
        while condition >= 2 {
            let node = &self.nodes[condition - 2];
            condition = node.children[assignment[node.variable]];
        }
        condition == ALWAYS
    }

    pub(super) fn witness(&self, mut condition: Condition) -> Option<Vec<usize>> {
        if condition == NEVER {
            return None;
        }
        let mut assignment = vec![0; self.widths.len()];
        while condition >= 2 {
            let node = &self.nodes[condition - 2];
            let branch = node
                .children
                .iter()
                .position(|&child| child != NEVER)
                .expect("a reduced condition has a successor");
            assignment[node.variable] = branch;
            condition = node.children[branch];
        }
        Some(assignment)
    }

    pub(super) fn count(&self, condition: Condition) -> usize {
        self.count_up_to(condition, None)
    }

    pub(super) fn count_up_to(&self, condition: Condition, limit: Option<usize>) -> usize {
        fn bounded(count: Option<usize>, limit: Option<usize>) -> usize {
            let count = count.unwrap_or_else(|| limit.expect("the execution count exceeds usize"));
            limit.map_or(count, |limit| count.min(limit))
        }
        fn visit(
            c: &Conditions,
            id: Condition,
            from: usize,
            limit: Option<usize>,
            memo: &mut HashMap<(Condition, usize), usize>,
        ) -> usize {
            if id == NEVER {
                return 0;
            }
            if let Some(&count) = memo.get(&(id, from)) {
                return count;
            }
            let until = c.variable(id).min(c.widths.len());
            let factor = (from..until)
                .filter(|v| v % 2 == 0)
                .fold(1usize, |n, v| bounded(n.checked_mul(c.widths[v]), limit));
            let rest = if id == ALWAYS {
                1
            } else {
                c.nodes[id - 2].children.iter().fold(0usize, |n, &child| {
                    bounded(
                        n.checked_add(visit(c, child, until + 1, limit, memo)),
                        limit,
                    )
                })
            };
            let result = bounded(factor.checked_mul(rest), limit);
            memo.insert((id, from), result);
            result
        }
        visit(self, condition, 0, limit, &mut HashMap::new())
    }
}
