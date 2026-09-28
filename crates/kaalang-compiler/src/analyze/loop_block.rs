//! Enters one cycle iteration; its `continue` records a repeat.

use syn::{Error, Result};

use super::frame::Frames;
use super::{LoopState, State, Walk};
use crate::model::{ExecutionOutcome, Flow};

/// The body inherits every outer wire; completion drops its locals again.
pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    let outside = LoopState {
        available: state.available.clone(),
        produced: state.produced.clone(),
    };
    state.loops.insert(block, outside);
    walk.visit(block + 1, state);
}

/// A selection stops governing its iteration's branches when that cycle completes.
/// Reaching a later block depends on completing the cycle. This is control
/// order, not a capture dependency or a wire merge. A cycle with several
/// outputs selects one as it completes, so its own selection stays open.
pub(super) fn closed_before(flow: &Flow, selection: usize, next: usize) -> bool {
    flow.enclosing(selection)
        .any(|index| flow.blocks[index].loop_end.is_some_and(|end| end <= next))
}

/// A cycle's outputs leave it left to right in declaration order, so the routes
/// exporting them keep that order among the body's branches.
/// Its back edge climbs one flank, so no repeating route lies between two of
/// them.
pub(super) fn output_order(flow: &Flow, frames: &Frames<'_>) -> Result<()> {
    // A nested cycle ends before the one around it, so its own order is
    // reported first rather than as a disorder of the enclosing cycle.
    let mut headers = (0..flow.blocks.len())
        .filter(|&header| flow.blocks[header].has_alternative_outputs())
        .collect::<Vec<_>>();
    headers.sort_by_key(|&header| flow.blocks[header].loop_end);
    for header in headers {
        let cycle = &flow.blocks[header];
        let exports = flow.exports(header);
        // Each route that reaches this cycle's end or its continue, with the
        // output it exports; `None` repeats the cycle.
        let (executions, outcomes): (Vec<_>, Vec<_>) = frames
            .view(Some(header))
            .iter()
            .filter_map(|execution| {
                let output = exports
                    .clone()
                    .position(|consumer| execution.participates(consumer));
                let repeats = execution.outcome == ExecutionOutcome::Repeat { loop_index: header };
                (output.is_some() || repeats).then_some((execution, output))
            })
            .unzip();
        let ordered = super::branch_order(&executions, &outcomes);
        let outputs = ordered
            .iter()
            .filter_map(|&index| outcomes[index])
            .collect::<Vec<_>>();
        let first = ordered.iter().position(|&index| outcomes[index].is_some());
        let last = ordered.iter().rposition(|&index| outcomes[index].is_some());
        if let (Some(first), Some(last)) = (first, last)
            && let Some(between) = (first..last).find(|&at| outcomes[ordered[at]].is_none())
        {
            let before = outcomes[ordered[between - 1]].expect("an output precedes the repeat");
            let after = ordered[between..]
                .iter()
                .find_map(|&index| outcomes[index])
                .expect("an output follows the repeat");
            return Err(Error::new(
                cycle.span,
                format!(
                    "a route repeating this kaalang cycle lies between routes exporting `{}` and `{}`; move the repeating routes to one edge of the body",
                    cycle.output_binding(before).ident,
                    cycle.output_binding(after).ident
                ),
            ));
        }
        // `drawn_first` leaves to the left of `drawn_later` yet is declared after it.
        if let Some((drawn_first, drawn_later)) = outputs
            .windows(2)
            .map(|pair| (pair[0], pair[1]))
            .find(|(first, later)| first > later)
        {
            return Err(Error::new(
                cycle.output_binding(drawn_first).ident.span(),
                format!(
                    "the routes exporting `{}` leave this kaalang cycle to the left of those exporting `{}`; declare its outputs in the order their routes leave it",
                    cycle.output_binding(drawn_first).ident,
                    cycle.output_binding(drawn_later).ident
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn moving_the_repeating_case_to_either_edge_is_valid() {
        let source = include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat.rs");
        for outputs in ["(advance, first, last)", "(first, last, advance)"] {
            let source = source.replace("(first, advance, last)", outputs);
            crate::build(&crate::tests::fixture(&source, "invalid"))
                .expect("adjacent exits leave the return an outer contour");
        }
        let source = r#"
            #[kaalang]
            fn invalid(mut mode: u8) -> u8 {
                #[cycle("Select an exit from a nested cycle.")]
                let selected = {
                    #[cycle("Advance at most once.")]
                    let selected = {
                        #[question("Exit immediately?")]
                        let (done, check) = |mode| mode == 0;

                        #[question("Advance once?")]
                        let (done, advance) = |check, mode| mode == 1;

                        #[action("Advance to the final case.")]
                        |advance, &mut mode| *mode = 2;

                        #[action("Keep the mode.")]
                        let selected = |done, mode| mode;
                        |advance| continue;
                    };
                };

                |selected| return selected;
            }
        "#;
        crate::build(&crate::tests::fixture(source, "invalid"))
            .expect("nested questions can place their exits next to each other");
    }

    #[test]
    fn a_converged_selection_does_not_split_loop_exit_routes() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/multiple_exits.rs")
            .replace(
                "    let selected = {",
                r#"    let selected = {
        #[question("Prepare this iteration?")]
        let (left, right) = |mode| mode == 0;

        #[action("Prepare the left branch.")]
        let ready = |left| ();

        #[action("Prepare the right branch.")]
        let ready = |right| ();

        #[action("Finish the shared preparation.")]
        |ready| {};
"#,
            );
        crate::build(&crate::tests::fixture(&source, "multiple_exits"))
            .expect("completed branches do not duplicate the following exit routes");
    }

    #[test]
    fn a_partial_merge_before_loop_entry_does_not_split_its_exit_routes() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/conditional_entry.rs")
            .replace(
                "#[question(\"Run the counter?\")]\n    let (run, skip) = |enabled| enabled;",
                r#"#[choice("Run the counter?")]
    #[case("Run when enabled.")]
    #[case("Run for a zero limit.")]
    #[case("Skip the counter.")]
    let (first, second, skip) = |enabled, limit| match (enabled, limit) {
        (true, _) => (),
        (false, 0) => (),
        _ => (),
    };

    #[action("Run when enabled.")]
    let run = |first| ();

    #[action("Run for a zero limit.")]
    let run = |second| ();"#,
            );
        crate::build(&crate::tests::fixture(&source, "conditional_entry"))
            .expect("routes skipping the loop do not decide its exit order");
    }
}
