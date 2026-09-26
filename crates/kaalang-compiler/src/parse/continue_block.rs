//! Parses the transfer that repeats the current cycle.

use syn::{Error, ExprContinue, Result, spanned::Spanned};

use super::structural_block;
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(
    expression: &ExprContinue,
    inputs: Vec<Input>,
    parent: Option<usize>,
    preceding: &[Block],
) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang continue does not support attributes",
        ));
    }
    if let Some(label) = &expression.label {
        return Err(Error::new_spanned(
            label,
            "a kaalang continue does not support labels; it repeats the current cycle",
        ));
    }
    let Some(target) = parent else {
        return Err(Error::new_spanned(
            expression,
            "a kaalang continue requires an enclosing cycle",
        ));
    };
    if preceding
        .iter()
        .any(|block| block.kind == BlockKind::Continue && block.parent == Some(target))
    {
        return Err(Error::new(
            expression.span(),
            "a kaalang cycle may declare at most one structural `continue`; merge its repeating routes before that continue",
        ));
    }
    Ok(structural_block(
        BlockKind::Continue,
        expression.span(),
        inputs,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_cycle_rejects_a_second_continue_in_every_spelling() {
        for transfer in ["continue;", "|| continue;", "|| { continue; };"] {
            let source =
                format!("fn example() {{ #[cycle(\"Repeat.\")] {{ {transfer} {transfer} }}; }}");
            let error = crate::parse::flow(&syn::parse_str(&source).unwrap())
                .err()
                .expect("a second structural continue is rejected before reachability");
            assert_eq!(
                error.to_string(),
                "a kaalang cycle may declare at most one structural `continue`; merge its repeating routes before that continue",
            );
        }
    }

    #[test]
    fn nested_and_native_cycles_do_not_share_the_continue_limit() {
        let function = syn::parse_quote! {
            fn example() {
                #[cycle("Repeat the outer cycle.")]
                {
                    #[cycle("Repeat the inner cycle.")]
                    {
                        continue;
                    };
                    #[action("Use native Rust control flow.")]
                    {
                        for _ in 0..2 {
                            continue;
                        }
                    };
                    continue;
                };
            }
        };
        assert!(crate::parse::flow(&function).is_ok());
    }
}
