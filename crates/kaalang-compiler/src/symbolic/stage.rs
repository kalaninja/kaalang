//! Preparation's common outer data.

use proc_macro2::Ident;

use super::{Executions, condition::NEVER};
use crate::{ExecutionOutcome, Flow, ProducerId};

impl Executions {
    pub(crate) fn available_on_completion(&mut self, flow: &Flow, name: &Ident) -> bool {
        if flow.flow_inputs.contains(name) {
            return true;
        }
        let completed = self
            .outcomes
            .iter()
            .filter(|(outcome, _)| matches!(outcome, ExecutionOutcome::Return { .. }))
            .fold(NEVER, |sum, (_, &when)| self.conditions.or(sum, when));
        let completed = self.conditions.and(completed, self.domain);
        let mut present = NEVER;
        for (block, declaration) in flow
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.parent.is_none())
        {
            for (output, _) in declaration
                .outputs
                .iter()
                .enumerate()
                .filter(|(_, wire)| *wire == name)
            {
                present = self.conditions.or(
                    present,
                    self.produced(ProducerId::BlockOutput { block, output }),
                );
            }
        }
        self.conditions.minus(completed, present) == NEVER
    }
}
