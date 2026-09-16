//! Parses a value-bearing transfer from the current cycle.

use syn::{Error, ExprBreak, Result, spanned::Spanned};

use super::{structural_block, transfer_value};
use crate::model::{Block, BlockKind, Input};

pub(super) fn parse(
    expression: &ExprBreak,
    inputs: Vec<Input>,
    parent: Option<usize>,
    preceding: &[Block],
) -> Result<Block> {
    if let Some(attribute) = expression.attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "a kaalang break does not support attributes",
        ));
    }
    if let Some(label) = &expression.label {
        return Err(Error::new_spanned(
            label,
            "a kaalang break does not support labels; it completes the current cycle",
        ));
    }
    let Some(target) = parent else {
        return Err(Error::new_spanned(
            expression,
            "a kaalang break requires an enclosing cycle",
        ));
    };
    if preceding
        .iter()
        .any(|block| block.break_target == Some(target))
    {
        return Err(Error::new(
            expression.span(),
            "a kaalang cycle may declare at most one structural `break`; merge its exit routes before that break",
        ));
    }
    let value = transfer_value(
        expression.expr.as_deref(),
        expression.span(),
        &inputs,
        "break",
    )?;
    let mut block = structural_block(BlockKind::Break, expression.span(), inputs);
    block.body = value;
    block.break_target = Some(target);
    Ok(block)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_cycle_rejects_a_second_break_in_every_spelling() {
        for transfer in ["break;", "break ();", "|| break;", "|| { break; };"] {
            let source = format!(
                "fn example() {{ #[cycle(\"Finish.\")] || {{ {transfer} {transfer} }}; return; }}"
            );
            let error = crate::parse::flow(&syn::parse_str(&source).unwrap())
                .err()
                .expect("a second structural break is rejected before reachability");
            assert_eq!(
                error.to_string(),
                "a kaalang cycle may declare at most one structural `break`; merge its exit routes before that break",
            );
        }
    }

    #[test]
    fn nested_and_native_cycles_do_not_share_the_break_limit() {
        let function = syn::parse_quote! {
            fn example() {
                #[cycle("Finish the outer cycle.")]
                || {
                    #[cycle("Finish the inner cycle.")]
                    || { break; };
                    #[action("Use native Rust control flow.")]
                    || { loop { if true { break; } break; } };
                    break;
                };
                return;
            }
        };
        assert!(crate::parse::flow(&function).is_ok());
    }
}
