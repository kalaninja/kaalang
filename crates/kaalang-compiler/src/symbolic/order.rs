//! Authored branch ordering, summarized without listing its ordered histories.

use std::collections::HashMap;

use super::{
    Executions,
    condition::{Condition, Conditions, NEVER},
};

#[derive(Clone, Copy, Default)]
pub(super) struct Sequence {
    pub(super) first: Option<usize>,
    pub(super) last: Option<usize>,
    prefix_missing: bool,
    suffix_missing: bool,
    pub(super) gap: bool,
    pub(super) gap_outputs: Option<(usize, usize)>,
    pub(super) descent: Option<(usize, usize)>,
}

impl Sequence {
    fn append(self, next: Self) -> Self {
        Self {
            first: self.first.or(next.first),
            last: next.last.or(self.last),
            prefix_missing: self.prefix_missing || (self.first.is_none() && next.prefix_missing),
            suffix_missing: next.suffix_missing || (next.last.is_none() && self.suffix_missing),
            gap: self.gap
                || next.gap
                || (self.last.is_some()
                    && next.first.is_some()
                    && (self.suffix_missing || next.prefix_missing)),
            gap_outputs: self
                .gap_outputs
                .or_else(|| {
                    self.last
                        .zip(next.first)
                        .filter(|_| self.suffix_missing || next.prefix_missing)
                })
                .or(next.gap_outputs),
            descent: self
                .descent
                .or_else(|| self.last.zip(next.first).filter(|(a, b)| a > b))
                .or(next.descent),
        }
    }
}

impl Executions {
    pub(super) fn ordered(&mut self, predicates: &[Condition], retained: &[usize]) -> Sequence {
        fn visit(
            c: &mut Conditions,
            predicates: &[Condition],
            variables: &[usize],
            position: usize,
            memo: &mut HashMap<(usize, Vec<Condition>), Sequence>,
        ) -> Sequence {
            if predicates.iter().all(|&when| when == NEVER) {
                return Sequence::default();
            }
            if position == variables.len() {
                let value = predicates
                    .iter()
                    .position(|&when| when != NEVER)
                    .expect("the trace has an outcome");
                debug_assert_eq!(predicates.iter().filter(|&&when| when != NEVER).count(), 1);
                return if value == 0 {
                    Sequence {
                        prefix_missing: true,
                        suffix_missing: true,
                        ..Sequence::default()
                    }
                } else {
                    Sequence {
                        first: Some(value - 1),
                        last: Some(value - 1),
                        ..Sequence::default()
                    }
                };
            }
            let key = (position, predicates.to_vec());
            if let Some(&saved) = memo.get(&key) {
                return saved;
            }
            let variable = variables[position];
            let mut result = Sequence::default();
            for branch in 0..c.widths[variable] {
                let restricted: Vec<_> = predicates
                    .iter()
                    .map(|&when| c.restrict(when, variable, branch))
                    .collect();
                let next = visit(c, &restricted, variables, position + 1, memo);
                result = result.append(next);
            }
            memo.insert(key, result);
            result
        }
        // Stable ties retain Execution's original selection ordering. Variables
        // irrelevant to outcomes share the same memoized suffix.
        let mut variables = retained
            .iter()
            .map(|&position| position * 2)
            .collect::<Vec<_>>();
        variables.extend(
            (0..self.selectors.len())
                .map(|position| position * 2)
                .filter(|variable| !retained.iter().any(|&position| position * 2 == *variable)),
        );
        visit(
            &mut self.conditions,
            predicates,
            &variables,
            0,
            &mut HashMap::new(),
        )
    }
}
