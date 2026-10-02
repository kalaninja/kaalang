//! What selections each block sees. To its containing sequence a cycle with
//! several outputs is a branching block whose body is a black box: only the
//! output it exported leaves that body, and the route taken inside it only
//! tells executions apart at the cycle. Inside the body its own selection does not exist yet, since
//! the body's routes are what make it.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::model::{BranchSelection, Execution, Flow, Passes, ProducerId, WireMerge};

/// The executions as each frame of the flow sees them. A frame is the
/// innermost cycle with several outputs around a block, or the root.
pub(super) struct Frames<'a> {
    /// Per block, the innermost cycle with several outputs strictly around it.
    inner: Vec<Option<usize>>,
    /// Per frame, the executions with the selections that frame cannot see
    /// removed, in the original order. Only the root exists, borrowed, when no
    /// cycle has several outputs.
    views: BTreeMap<Option<usize>, Cow<'a, [Execution]>>,
    /// Which blocks repeating routes reach. The views keep every execution's
    /// blocks, so one table serves them all.
    passes: Passes,
}

impl<'a> Frames<'a> {
    pub(super) fn of(flow: &Flow, executions: &'a [Execution]) -> Self {
        let inner = (0..flow.blocks.len())
            .map(|block| {
                flow.enclosing(block)
                    .find(|&header| flow.blocks[header].has_alternative_outputs())
            })
            .collect::<Vec<_>>();
        let headers = (0..flow.blocks.len())
            .filter(|&block| flow.blocks[block].has_alternative_outputs())
            .collect::<Vec<_>>();
        let views = if headers.is_empty() {
            BTreeMap::from([(None, Cow::Borrowed(executions))])
        } else {
            std::iter::once(None)
                .chain(headers.iter().copied().map(Some))
                .map(|frame| {
                    let mut view = executions
                        .iter()
                        .map(|execution| {
                            let mut seen = execution.clone();
                            seen.branches
                                .retain(|selection| visible(&inner, frame, selection.block));
                            seen
                        })
                        .collect::<Vec<_>>();
                    for &header in &headers {
                        if visible(&inner, frame, header) {
                            mark_routes(flow, executions, &mut view, header);
                        }
                    }
                    (frame, Cow::Owned(view))
                })
                .collect()
        };
        Self {
            inner,
            views,
            passes: Passes::of(flow, executions),
        }
    }

    /// Which blocks repeating executions reach, for [`Passes::reaches`].
    pub(super) fn passes(&self) -> &Passes {
        &self.passes
    }

    pub(super) fn into_passes(self) -> Passes {
        self.passes
    }

    /// Whether every block sees every selection: no cycle has several outputs.
    pub(super) fn is_flat(&self) -> bool {
        self.views.len() == 1
    }

    /// The frame a merge's producers share.
    pub(super) fn of_merge(&self, merge: &WireMerge) -> Option<usize> {
        merge.producers.iter().find_map(|producer| match producer {
            ProducerId::BlockOutput { block, .. } => Some(self.frame(*block)),
            ProducerId::FlowInput(_) => None,
        })?
    }

    /// The innermost cycle with several outputs around `block`, or the root.
    pub(super) fn frame(&self, block: usize) -> Option<usize> {
        self.inner[block]
    }

    /// The executions as `frame` sees them.
    pub(super) fn view(&self, frame: Option<usize>) -> &[Execution] {
        &self.views[&frame]
    }

    /// The executions as `block` sees them.
    pub(super) fn at(&self, block: usize) -> &[Execution] {
        self.view(self.frame(block))
    }

    /// Whether a selection made at `block` is visible from `frame`.
    pub(super) fn visible(&self, frame: Option<usize>, block: usize) -> bool {
        visible(&self.inner, frame, block)
    }
}

/// How many body routes of one cycle a frame view tells apart; see
/// [`mark_routes`].
const ROUTES: usize = 1 << 20;

/// The branch a selection names, without the body route that a cycle seen as a
/// black box carries in it (see [`mark_routes`]). Every other selection is
/// below `ROUTES` and names its branch directly.
pub(super) const fn branch(selection: usize) -> usize {
    if selection < ROUTES {
        selection
    } else {
        selection / ROUTES - 1
    }
}

/// Folds the route an execution takes through the body of a cycle that `view`
/// sees as a black box into the cycle's own selection: output `k` reached by
/// route `r` becomes branch `(k + 1) * ROUTES + r`, and a route that repeats
/// or diverges selects one past the last output. Two executions that differ
/// inside the body then differ at the cycle, never only at a selection outside
/// it, just as they would with the body's selections in sight.
fn mark_routes(flow: &Flow, executions: &[Execution], view: &mut [Execution], header: usize) {
    let end = flow.blocks[header].cycle_end.expect("a cycle owns a body");
    let outputs = flow.blocks[header].outputs.len();
    let mut routes = BTreeMap::new();
    for (seen, execution) in view.iter_mut().zip(executions) {
        if !execution.participates(header) {
            continue;
        }
        let inside = execution
            .branches
            .iter()
            .filter(|selection| (header + 1..end).contains(&selection.block))
            .copied()
            .collect::<Vec<_>>();
        let next = routes.len();
        let route = *routes.entry(inside).or_insert(next);
        assert!(
            route < ROUTES,
            "a cycle body has too many routes to tell apart"
        );
        let selection = BranchSelection {
            block: header,
            branch: (execution.selected(header).unwrap_or(outputs) + 1) * ROUTES + route,
        };
        match seen
            .branches
            .binary_search_by_key(&header, |selection| selection.block)
        {
            Ok(position) => seen.branches[position] = selection,
            Err(position) => seen.branches.insert(position, selection),
        }
    }
}

/// Whether a selection made at `block` is visible from `frame`: it is not made
/// inside a cycle with several outputs that `frame` lies outside, and it is not
/// the own selection of one that `frame` lies inside.
pub(crate) fn visible(inner: &[Option<usize>], frame: Option<usize>, block: usize) -> bool {
    let around = || std::iter::successors(frame, |&header| inner[header]);
    inner[block].is_none_or(|header| around().any(|each| each == header))
        && !around().any(|each| each == block)
}

#[cfg(test)]
mod tests {
    use super::Frames;

    /// The body sees its own selections but not the cycle's, and the
    /// containing sequence sees the cycle's but not the body's.
    #[test]
    fn a_cycle_with_several_outputs_splits_what_each_side_sees() {
        let model = crate::build(&crate::tests::fixture(
            include_str!("../../../kaalang/tests/cycle/behavior/alternative_outputs.rs"),
            "alternative_outputs",
        ))
        .expect("the fixture is valid");
        let frames = Frames::of(&model.analysis.flow, &model.analysis.executions);
        let (cycle, choice) = (0, 1);
        assert_eq!(frames.frame(3), Some(cycle), "a body action");
        assert_eq!(frames.frame(7), None, "an action after the cycle");
        assert!(!frames.visible(Some(cycle), cycle));
        assert!(frames.visible(Some(cycle), choice));
        assert!(frames.visible(None, cycle));
        assert!(!frames.visible(None, choice));
    }

    /// Outside the cycle, two routes to one output still tell executions
    /// apart, but only at the cycle: with the question above it they differ
    /// in two places, so that question alone decides nothing inside the body.
    #[test]
    fn body_routes_differ_only_at_their_cycle() {
        let model = crate::build(&crate::tests::fixture(
            include_str!("../../../kaalang/tests/cycle/behavior/output_routes_after_a_merge.rs"),
            "output_routes_after_a_merge",
        ))
        .expect("the fixture is valid");
        let flow = &model.analysis.flow;
        let frames = Frames::of(flow, &model.analysis.executions);
        let (question, twice, cycle, top, enough) = (0, 1, 3, 5, 6);
        let root = frames.view(None);
        let find = |factor: bool, route: usize| {
            root.iter()
                .find(|execution| {
                    execution.participates(twice) == factor && execution.participates(route)
                })
                .expect("every factor meets every passing route")
        };
        let only = crate::analyze::only_difference;
        assert_eq!(only(find(true, top), find(true, enough)), Some(cycle));
        assert_eq!(only(find(true, top), find(false, top)), Some(question));
        assert_eq!(only(find(true, top), find(false, enough)), None);
        assert_eq!(
            super::branch(find(true, enough).selected(cycle).unwrap()),
            0
        );
    }
}
