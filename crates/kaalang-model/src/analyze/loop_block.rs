//! Enters one unconditional iteration; reaching its end records a repeat.

use std::collections::BTreeSet;

use syn::{Error, Result};

use super::{State, Walk};
use crate::model::{Execution, Flow};

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, mut state: State) {
    state.loop_inputs.insert(block, state.available.clone());
    walk.visit(block + 1, state);
}

/// A loop's own return cannot lie between body routes that break to the same
/// target. Nested loops validate their own returns; their normal exits converge
/// before the containing iteration continues.
pub(super) fn validate_routes(flow: &Flow, executions: &[Execution]) -> Result<()> {
    for (header, declaration) in flow.blocks.iter().enumerate() {
        let Some(end) = declaration.loop_end else {
            continue;
        };
        if !executions
            .iter()
            .any(|execution| execution.repeats.contains(&header))
        {
            continue;
        }
        let targets = flow.blocks[header + 1..end]
            .iter()
            .filter_map(|block| block.break_target)
            .filter(|&target| target <= header)
            .collect::<BTreeSet<_>>();
        for target in targets {
            let (routes, exits): (Vec<_>, Vec<_>) = executions
                .iter()
                .filter_map(|execution| {
                    if execution.blocks.iter().any(|&block| {
                        (header + 1..end).contains(&block)
                            && flow.blocks[block].break_target == Some(target)
                    }) {
                        Some((execution, true))
                    } else if execution.repeats.contains(&header) {
                        Some((execution, false))
                    } else {
                        None
                    }
                })
                .unzip();
            let ordered = super::branch_order(&routes, &exits);
            let first = ordered.iter().position(|&index| exits[index]);
            let last = ordered.iter().rposition(|&index| exits[index]);
            if let (Some(first), Some(last)) = (first, last)
                && ordered[first..=last].iter().any(|&index| !exits[index])
            {
                return Err(Error::new(
                    declaration.span,
                    "a repeating kaalang branch cannot lie between breaks to the same loop; reorder the branches",
                ));
            }
        }
    }
    Ok(())
}

/// A selection stops governing its iteration's branches when that loop ends.
/// Reaching a later block depends on leaving the loop normally, including when
/// a nested selection could have returned `end`. This is control order, not
/// a capture dependency or a wire merge.
pub(super) fn closed_before(flow: &Flow, selection: usize, next: usize) -> bool {
    std::iter::once(selection)
        .chain(flow.enclosing(selection))
        .any(|index| flow.blocks[index].loop_end.is_some_and(|end| end <= next))
}

#[cfg(test)]
mod tests {
    use syn::{Block, ItemFn, Stmt, parse_quote};

    #[test]
    fn moving_the_repeating_case_to_either_edge_is_valid() {
        let source = include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat.rs");
        for outputs in ["(advance, first, last)", "(first, last, advance)"] {
            let source = source.replace("(first, advance, last)", outputs);
            crate::build(&crate::tests::fixture(&source, "invalid"))
                .expect("adjacent exits leave the return an outer contour");
        }
        let source =
            include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat_nested.rs")
                .replace("(advance, last)", "(last, advance)");
        crate::build(&crate::tests::fixture(&source, "invalid"))
            .expect("nested questions can place their exits next to each other");
    }

    #[test]
    fn a_converged_selection_does_not_split_loop_exit_routes() {
        let source = include_str!("../../../kaalang/tests/loop/behavior/multiple_exits.rs")
            .replace(
                "    loop {",
                r#"    loop {
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

    #[test]
    fn early_end_wires_do_not_make_later_selections_independent() {
        let prefixes: [Stmt; 2] = [
            parse_quote! {
                |&stop| loop {
                    #[question("Finish early?")]
                    let (iterate_1, leave_1) = |stop| stop;
                    |leave_1| break;
                        #[action("Return early.")]
                        let end = |iterate_1| 99;
                };
            },
            parse_quote! {
                |&stop| loop {
                    #[question("Check for an early result?")]
                    let (iterate_2, leave_2) = |stop| stop;
                    |leave_2| break;
                        // The repeating answer comes first, so the return
                        // keeps an outer contour: a route that finishes the
                        // flow from inside the body cannot sit between the
                        // break and the repeat (RFC 0002 §8).
                        #[question("Is the count nonzero?")]
                        let (resume, finish) = |iterate_2, count| count != 0;
                        #[action("Resume after the loop.")]
                        |resume, &mut stop| *stop = false;
                        #[action("Return early.")]
                        let end = |finish| 99;
                };
            },
        ];
        let suffixes: [Block; 3] = [
            parse_quote! {
                {
                    |&count| loop {
                        #[question("Count up to three?")]
                        let (iterate_3, leave_3) = |count| count < 3;
                        |leave_3| break;
                            #[action("Increment.")]
                            |iterate_3, &mut count| *count += 1;
                    };
                    #[action("Finish.")]
                    let end = |count| count;
                }
            },
            parse_quote! {
                {
                    #[question("Is the count zero?")]
                    let (zero, nonzero) = |count| count == 0;
                    #[action("Report zero.")]
                    let end = |zero| 0;
                    #[action("Report the count.")]
                    let end = |nonzero, count| count;
                }
            },
            parse_quote! {
                {
                    #[choice("Which count?")]
                    #[case("Zero.")]
                    #[case("Nonzero.")]
                    let (zero, nonzero) = |count| match count { 0 => (), _ => () };
                    #[action("Report zero.")]
                    let end = |zero| 0;
                    #[action("Report the count.")]
                    let end = |nonzero, count| count;
                }
            },
        ];
        for prefix in prefixes {
            for suffix in &suffixes {
                let mut function: ItemFn = parse_quote! {
                    fn example(mut stop: bool, mut count: usize) -> usize {}
                };
                function.block.stmts.push(prefix.clone());
                function.block.stmts.extend(suffix.stmts.iter().cloned());
                let model =
                    crate::build(&function).expect("normal loop exit admits a later selection");
                assert!(
                    model.convergence_groups.is_empty(),
                    "loop exit order adds no capture-based convergence group"
                );
            }
        }
    }
}
