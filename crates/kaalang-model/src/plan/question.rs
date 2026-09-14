//! Builds a question's nested lowering plan.

use crate::model::{Branch, ExecutionPlan, Join};

pub(super) fn dispatch(index: usize, branches: Vec<Branch>, joins: Vec<Join>) -> ExecutionPlan {
    let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
        unreachable!("a question declares exactly two outputs")
    };
    ExecutionPlan::Question {
        index,
        branches,
        joins,
    }
}
