//! Builds a question's nested lowering plan.

use crate::model::{Branch, ExecutionPlan, Join};

pub(super) fn dispatch(index: usize, branches: Vec<Branch>, joins: Vec<Join>) -> ExecutionPlan {
    ExecutionPlan::Question {
        index,
        branches,
        joins,
    }
}
