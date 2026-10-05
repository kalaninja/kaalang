//! One source-order traversal of finite cycle frames and transition boundaries.

use std::collections::BTreeMap;

use proc_macro2::Ident;
use syn::{Error, Result};

use super::{
    Executions,
    condition::{ALWAYS, Condition, Conditions, NEVER},
};
use crate::{BlockKind, CaptureDependency, CaptureId, ExecutionOutcome, Flow, ProducerId};

mod action;
mod call;
mod choice;
mod continue_block;
mod cycle;
mod export;
mod question;
mod return_block;

type Available = BTreeMap<Ident, Vec<(ProducerId, Condition)>>;

struct State {
    live: Condition,
    available: Available,
    /// Routes that pass a span of blocks by, each rejoining at its block.
    resume: Vec<(usize, Condition)>,
}

struct Walk<'a> {
    flow: &'a Flow,
    executions: Executions,
    exports: Vec<Condition>,
    error: Option<((usize, usize), Error)>,
}

pub(crate) fn flow(flow: &Flow) -> Result<Executions> {
    let selectors: Vec<_> = flow
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(block, declaration)| (declaration.branch_count() > 0).then_some(block))
        .collect();
    let widths = selectors
        .iter()
        .flat_map(|&block| [flow.blocks[block].outputs.len() + 1; 2])
        .collect();
    let mut available = Available::new();
    let mut outputs = BTreeMap::new();
    for (input, name) in flow.flow_inputs.iter().enumerate() {
        let producer = ProducerId::FlowInput(input);
        available.insert(name.clone(), vec![(producer, ALWAYS)]);
        outputs.insert(producer, ALWAYS);
    }
    let mut walk = Walk {
        flow,
        executions: Executions {
            conditions: Conditions::new(widths),
            domain: ALWAYS,
            runs: vec![NEVER; flow.blocks.len()],
            selected_runs: vec![NEVER; flow.blocks.len()],
            produced: outputs,
            dependencies: BTreeMap::new(),
            outcomes: BTreeMap::new(),
            selectors,
        },
        exports: vec![NEVER; flow.blocks.len()],
        error: None,
    };
    let end = flow.blocks.len() - 1;
    let state = walk.sequence(
        0,
        end,
        State {
            live: ALWAYS,
            available,
            resume: Vec::new(),
        },
    );
    walk.executions.runs[end] = state.live;
    if let Some((_, error)) = walk.error {
        return Err(error);
    }
    Ok(walk.executions)
}

impl Walk<'_> {
    fn report(&mut self, key: (usize, usize), error: Error) {
        if self.error.as_ref().is_none_or(|(old, _)| key < *old) {
            self.error = Some((key, error));
        }
    }

    fn present(&mut self, available: &Available, name: &Ident) -> Condition {
        available
            .get(name)
            .into_iter()
            .flatten()
            .fold(NEVER, |sum, &(_, when)| {
                self.executions.conditions.or(sum, when)
            })
    }

    /// A transition boundary selects at most one stage signal, and each common
    /// outer wire it carries is available on every route reaching it.
    fn transition(&mut self, index: usize, state: &State) -> Result<()> {
        let flow = self.flow;
        let block = &flow.blocks[index];
        if block.transition_target.is_none() {
            return Ok(());
        }
        let context = self
            .executions
            .conditions
            .and(self.executions.domain, state.live);
        let mut previous = NEVER;
        for candidate in flow
            .blocks
            .iter()
            .filter(|candidate| candidate.transition_target.is_some())
        {
            let selected = self.present(&state.available, &candidate.inputs[0].ident);
            let c = &mut self.executions.conditions;
            let both = c.and(previous, selected);
            if c.and(context, both) != NEVER {
                return Err(Error::new(
                    candidate.span,
                    "a kaalang transition boundary selects more than one stage signal",
                ));
            }
            previous = c.or(previous, selected);
        }
        for input in block.inputs.iter().skip(1) {
            let provided = self.present(&state.available, &input.ident);
            if self.executions.conditions.minus(context, provided) != NEVER {
                return Err(Error::new(
                    input.alias.span(),
                    "a common outer wire used by a kaalang stage must be available on every preparation route",
                ));
            }
        }
        Ok(())
    }

    fn has(&mut self, when: Condition) -> bool {
        self.executions.has(when)
    }

    fn constrain(&mut self, block: usize, active: Condition) {
        self.executions.selected_runs[block] = active;
        if let Ok(position) = self.executions.selectors.binary_search(&block) {
            let c = &mut self.executions.conditions;
            let absent = c.selected(position * 2, self.flow.blocks[block].outputs.len());
            let inactive = c.minus(absent, active);
            let selected = c.minus(active, absent);
            let allowed = c.or(selected, inactive);
            self.executions.domain = c.and(self.executions.domain, allowed);
        }
    }

    fn produce(
        &mut self,
        available: &mut Available,
        block: usize,
        output: usize,
        mut when: Condition,
    ) -> Condition {
        let name = &self.flow.blocks[block].outputs[output];
        let previous = self.present(available, name);
        let duplicate = self.executions.conditions.and(previous, when);
        if self.has(duplicate) {
            self.report((block, output), crate::analyze::produced_twice(name));
            when = self.executions.conditions.minus(when, duplicate);
        }
        let producer = ProducerId::BlockOutput { block, output };
        available
            .entry(name.clone())
            .or_default()
            .push((producer, when));
        self.executions.produced.insert(producer, when);
        when
    }

    fn enter(&mut self, block: usize, state: &mut State) -> Condition {
        let declaration = &self.flow.blocks[block];
        let mut runs = state.live;
        for input in declaration.inputs.iter().filter(|input| !input.derived) {
            let provided = self.present(&state.available, &input.ident);
            runs = self.executions.conditions.and(runs, provided);
        }
        if declaration.kind == BlockKind::Cycle {
            for (position, input) in declaration
                .inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| input.derived)
            {
                let provided = self.present(&state.available, &input.ident);
                let absent = self.executions.conditions.minus(runs, provided);
                if self.has(absent) {
                    self.report((block, position), Error::new(input.ident.span(), "an outer wire captured inside a kaalang cycle must be available whenever the cycle is entered"));
                    runs = self.executions.conditions.minus(runs, absent);
                    state.live = self.executions.conditions.minus(state.live, absent);
                }
            }
        }
        self.executions.runs[block] = runs;
        for (input, declaration) in declaration.inputs.iter().enumerate() {
            for &(producer, when) in state
                .available
                .get(&declaration.ident)
                .into_iter()
                .flatten()
            {
                let when = self.executions.conditions.and(runs, when);
                if when != NEVER {
                    self.executions.dependencies.insert(
                        CaptureDependency {
                            producer,
                            capture: CaptureId { block, input },
                        },
                        when,
                    );
                }
            }
        }
        runs
    }

    fn finish(&mut self, state: &mut State, outcome: ExecutionOutcome, when: Condition) {
        let entry = self.executions.outcomes.entry(outcome).or_insert(NEVER);
        *entry = self.executions.conditions.or(*entry, when);
        state.live = self.executions.conditions.minus(state.live, when);
    }

    fn sequence(&mut self, start: usize, end: usize, mut state: State) -> State {
        let mut block = start;
        while block < end {
            for (_, when) in state.resume.extract_if(.., |(at, _)| *at == block) {
                state.live = self.executions.conditions.or(state.live, when);
            }
            if let Err(error) = self.transition(block, &state) {
                self.report((block, 0), error);
                state.live = NEVER;
            }
            let runs = self.enter(block, &mut state);
            let declaration = &self.flow.blocks[block];
            match declaration.kind {
                BlockKind::Cycle => {
                    state = cycle::visit(self, block, state, runs);
                    block = declaration.cycle_end.expect("a cycle owns a body");
                    continue;
                }
                BlockKind::Export => export::visit(self, block, &mut state, runs),
                BlockKind::Continue => continue_block::visit(self, block, &mut state, runs),
                BlockKind::Return => return_block::visit(self, block, &mut state, runs),
                BlockKind::Question => question::visit(self, block, &mut state, runs),
                BlockKind::Choice => choice::visit(self, block, &mut state, runs),
                BlockKind::Action => action::visit(self, block, &mut state, runs),
                BlockKind::Call => call::visit(self, block, &mut state, runs),
                BlockKind::End => unreachable!("the end closes the root sequence"),
            }
            block += 1;
        }
        state
    }
}
