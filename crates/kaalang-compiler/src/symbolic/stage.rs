//! Local transition boundaries and preparation's common outer data.

use std::collections::BTreeMap;

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{
    Executions,
    condition::{Condition, Conditions, NEVER},
};
use crate::{ExecutionOutcome, Flow, ProducerId};

pub(super) fn transition(
    conditions: &mut Conditions,
    flow: &Flow,
    index: usize,
    domain: Condition,
    live: Condition,
    available: &BTreeMap<Ident, Vec<(ProducerId, Condition)>>,
) -> Result<()> {
    let block = &flow.blocks[index];
    if block.transition_target.is_none() {
        return Ok(());
    }
    let context = conditions.and(domain, live);
    let present = |conditions: &mut Conditions, name: &Ident| {
        available
            .get(name)
            .into_iter()
            .flatten()
            .fold(NEVER, |sum, &(_, when)| conditions.or(sum, when))
    };
    let mut previous = NEVER;
    for candidate in flow
        .blocks
        .iter()
        .filter(|candidate| candidate.transition_target.is_some())
    {
        let selected = present(conditions, &candidate.inputs[0].ident);
        let both = conditions.and(previous, selected);
        if conditions.and(context, both) != NEVER {
            return Err(Error::new(
                candidate.span,
                "a kaalang transition boundary selects more than one stage signal",
            ));
        }
        previous = conditions.or(previous, selected);
    }
    for input in block.inputs.iter().skip(1) {
        let provided = present(conditions, &input.ident);
        if conditions.minus(context, provided) != NEVER {
            return Err(Error::new(
                input.alias.span(),
                "a common outer wire used by a kaalang stage must be available on every preparation route",
            ));
        }
    }
    Ok(())
}

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
