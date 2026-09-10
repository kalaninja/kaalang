//! Summarizes zero iterations and one possible iteration followed by exit.
//! Mutable values stay Rust's concern; availability is invariant at the header.

use crate::model::{BranchSelection, Flow};

use super::{State, Walk};

/// A selection stops governing its iteration's branches when that loop ends.
/// Reaching a later block depends on leaving the loop normally, including when
/// a nested selection could have returned `end`. This is control order, not
/// a capture dependency or a wire merge.
pub(super) fn closed_before(flow: &Flow, selection: usize, next: usize) -> bool {
    let mut owner = Some(selection);
    while let Some(index) = owner {
        if flow.blocks[index].loop_end.is_some_and(|end| end <= next) {
            return true;
        }
        owner = flow.blocks[index].parent;
    }
    false
}

pub(super) fn visit(walk: &mut Walk<'_>, block: usize, state: &State) {
    let declaration = &walk.flow.blocks[block];
    let end = declaration.loop_end.expect("a while owns a body region");
    for branch in 0..2 {
        let mut state = state.clone();
        state.branches.insert(BranchSelection { block, branch });
        if branch == declaration.yes_branch() {
            state.loop_inputs.insert(block, state.available.clone());
            walk.visit(block + 1, state);
        } else {
            walk.visit(end, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::{Block, ItemFn, Stmt, parse_quote};

    #[test]
    fn early_end_wires_do_not_make_later_selections_independent() {
        let prefixes: [Stmt; 2] = [
            parse_quote! {
                #[question("Finish early?")]
                while (|stop| stop) {
                    #[action("Return early.")]
                    let end = || 99;
                }
            },
            parse_quote! {
                #[question("Check for an early result?")]
                while (|stop| stop) {
                    #[question("Is the count zero?")]
                    let (finish, resume) = |count| count == 0;
                    #[action("Return early.")]
                    let end = |finish| 99;
                    #[action("Resume after the loop.")]
                    |resume, &mut stop| *stop = false;
                }
            },
        ];
        let suffixes: [Block; 3] = [
            parse_quote! {
                {
                    #[question("Count up to three?")]
                    while (|count| count < 3) {
                        #[action("Increment.")]
                        |&mut count| *count += 1;
                    }
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
