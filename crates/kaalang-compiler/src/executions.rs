//! Lazy materialization of the complete structural execution summaries.

use crate::Execution;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

/// Complete execution summaries, ordered as [`Execution`].
///
/// Compilation can retain exact conditions instead of listing every execution.
/// Access through the `Vec` interface enumerates and caches the complete list.
#[derive(Clone)]
pub struct Executions {
    storage: Storage,
}

#[derive(Clone)]
enum Storage {
    Enumerated(Vec<Execution>),
    Symbolic {
        conditions: Box<crate::symbolic::Executions>,
        observations: Vec<Execution>,
        complete: OnceLock<Vec<Execution>>,
    },
}

impl Executions {
    /// The number of executions, without materializing their summaries.
    ///
    /// # Panics
    ///
    /// Panics if the execution count exceeds `usize`.
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.storage {
            Storage::Enumerated(executions) => executions.len(),
            Storage::Symbolic { conditions, .. } => conditions.len(),
        }
    }

    /// Whether there are no executions, without materializing their summaries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(&self.storage, Storage::Enumerated(executions) if executions.is_empty())
    }

    pub(crate) fn enumerated(executions: Vec<Execution>) -> Self {
        Self {
            storage: Storage::Enumerated(executions),
        }
    }

    pub(crate) fn factored(mut conditions: crate::symbolic::Executions) -> Self {
        conditions.compact();
        let observations = conditions.observations();
        Self {
            storage: Storage::Symbolic {
                conditions: Box::new(conditions),
                observations,
                complete: OnceLock::new(),
            },
        }
    }

    pub(crate) fn summaries(&self) -> &[Execution] {
        match &self.storage {
            Storage::Enumerated(executions) => executions,
            Storage::Symbolic { observations, .. } => observations,
        }
    }

    pub(crate) fn symbolic(&self) -> Option<&crate::symbolic::Executions> {
        match &self.storage {
            Storage::Enumerated(_) => None,
            Storage::Symbolic { conditions, .. } => Some(conditions),
        }
    }

    pub(crate) fn common_on_completion(
        &self,
        flow: &crate::Flow,
        candidates: std::collections::BTreeSet<proc_macro2::Ident>,
    ) -> std::collections::BTreeSet<proc_macro2::Ident> {
        let mut symbolic = self.symbolic().cloned();
        candidates
            .into_iter()
            .filter(|name| {
                if let Some(conditions) = &mut symbolic {
                    conditions.available_on_completion(flow, name)
                } else {
                    self.summaries()
                        .iter()
                        .filter(|execution| {
                            matches!(execution.outcome, crate::ExecutionOutcome::Return { .. })
                        })
                        .all(|execution| crate::stage::available(flow, execution, name))
                }
            })
            .collect()
    }
}

impl Deref for Executions {
    type Target = Vec<Execution>;
    fn deref(&self) -> &Self::Target {
        match &self.storage {
            Storage::Enumerated(executions) => executions,
            Storage::Symbolic {
                conditions,
                complete,
                ..
            } => complete.get_or_init(|| conditions.enumerate()),
        }
    }
}

impl DerefMut for Executions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if matches!(self.storage, Storage::Symbolic { .. }) {
            let storage = std::mem::replace(&mut self.storage, Storage::Enumerated(Vec::new()));
            let Storage::Symbolic {
                conditions,
                complete,
                ..
            } = storage
            else {
                unreachable!()
            };
            self.storage = Storage::Enumerated(
                complete
                    .into_inner()
                    .unwrap_or_else(|| conditions.enumerate()),
            );
        }
        let Storage::Enumerated(executions) = &mut self.storage else {
            unreachable!()
        };
        executions
    }
}

impl<'a> IntoIterator for &'a Executions {
    type Item = &'a Execution;
    type IntoIter = std::slice::Iter<'a, Execution>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_branch_work_stays_factored_until_the_complete_list_is_requested() {
        let function =
            kaalang_testing::probes::flow(&kaalang_testing::probes::branching_with_work(5));
        let analysis = crate::analyze(&function).expect("the flow analyzes");
        assert_eq!(analysis.executions.len(), 72);
        assert!(
            matches!(&analysis.executions.storage, Storage::Symbolic { complete, .. } if complete.get().is_none())
        );
        let _ = crate::project(&analysis, false);
        assert!(
            matches!(&analysis.executions.storage, Storage::Symbolic { complete, .. } if complete.get().is_none())
        );
        let mut edited = analysis.executions.clone();
        edited.pop();
        assert_eq!(edited.len(), 71);
        assert!(edited.symbolic().is_none());
        assert_eq!(analysis.executions.as_slice().len(), 72);
    }

    #[test]
    fn stage_visitation_does_not_materialize_a_terminal_stages_histories() {
        let source = kaalang_testing::probes::branching_with_work(20);
        let body = source[source.find('{').expect("the probe has a body") + 1
            ..source.rfind('}').expect("the probe closes")]
            .replace("|seed| seed;", "|entry, seed| seed;");
        let staged = format!(
            "#[kaalang] fn staged(seed: usize, config: usize, events: &std::cell::Cell<usize>) -> usize {{\n#[action(\"Start.\")] let entry = || ();\n#[stage(\"Work.\")] |entry| {{ {body} }};\n}}"
        );
        let function = kaalang_testing::probes::flow(&staged);
        let analysis = crate::analyze(&function).expect("the staged flow analyzes");
        let executions = &analysis.stages[0].analysis.executions;
        assert_eq!(executions.len(), 60_466_176);
        assert!(
            matches!(&executions.storage, Storage::Symbolic { complete, .. } if complete.get().is_none())
        );
    }
    #[test]
    fn cycles_preparation_and_transitions_keep_large_histories_factored() {
        for source in [
            kaalang_testing::probes::cyclic_branching_with_work(20),
            kaalang_testing::probes::staged_branching_with_work(20),
        ] {
            let function = kaalang_testing::probes::flow(&source);
            let analysis = crate::analyze(&function).expect("the generated flow analyzes");
            let parts = std::iter::once(&analysis)
                .chain(analysis.stages.iter().map(|stage| &*stage.analysis));
            for part in parts {
                if part.executions.len() > 64 {
                    assert!(
                        matches!(&part.executions.storage, Storage::Symbolic { complete, .. } if complete.get().is_none())
                    );
                    assert!(matches!(part.executions.len(), 60_466_176 | 120_932_353));
                    for collapsed in [false, true] {
                        let _ = crate::project(part, collapsed);
                    }
                    assert!(
                        matches!(&part.executions.storage, Storage::Symbolic { complete, .. } if complete.get().is_none())
                    );
                }
            }
            assert!(
                !crate::expand(function)
                    .expect("the generated flow expands")
                    .block
                    .stmts
                    .is_empty()
            );
        }
    }
    #[test]
    fn small_cycle_domains_use_the_complete_strategy_after_counting() {
        let function =
            kaalang_testing::probes::flow(&kaalang_testing::probes::nested_cycles(8, 8, false));
        let analysis = crate::analyze(&function).expect("the probe analyzes");
        assert_eq!(analysis.executions.len(), 9);
        assert!(analysis.executions.symbolic().is_none());
    }
}
