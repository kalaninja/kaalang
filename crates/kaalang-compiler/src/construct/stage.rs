//! Stage transitions share one final row; branch routes decide their columns.

use std::collections::BTreeSet;

use super::Arrangement;
use crate::topology::{NodeKind, Topology, Vertex};

pub(super) fn events(topology: &Topology, events: &mut [Vec<usize>]) {
    let transitions = topology
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::StageTransition)
        .map(|node| super::index_of(topology, Vertex::Node(node.id)))
        .collect::<Vec<_>>();
    if let Some(&first) = transitions.first() {
        for &vertex in &transitions[1..] {
            events[vertex].clear();
        }
        events[first] = transitions;
    }
}

pub(super) fn verify(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    let transitions = topology
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::StageTransition)
        .map(|node| Vertex::Node(node.id))
        .collect::<BTreeSet<_>>();
    let Some(first) = transitions.first() else {
        return Ok(());
    };
    let Some(&row) = arrangement.rank.get(first) else {
        return Err(format!("{first:?} has no rank"));
    };
    let mut columns = BTreeSet::new();
    for vertex in &topology.vertices {
        let rank = arrangement.rank.get(vertex);
        if transitions.contains(vertex) {
            if rank != Some(&row) {
                return Err("stage transitions must share one row".to_owned());
            }
            let column = arrangement
                .column
                .get(vertex)
                .ok_or_else(|| format!("{vertex:?} has no column"))?;
            if !columns.insert(column) {
                return Err("stage transitions must occupy distinct columns".to_owned());
            }
        } else if rank.is_none_or(|&rank| rank >= row) {
            return Err("stage transitions must be below every other node and junction".to_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::NodeId;

    #[test]
    fn transitions_share_the_final_row_in_branch_order_in_both_constructions() {
        let function = crate::tests::fixture(
            include_str!("../../../kaalang/tests/stage/behavior/count_to.rs"),
            "count_to",
        );
        let model = crate::build(&function).unwrap();
        assert_eq!(model.topology.nodes[0].kind, NodeKind::Start);
        let stage = &model.stages[0];
        let topology = &stage.topology;
        let transitions = topology
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::StageTransition)
            .map(|node| Vertex::Node(node.id))
            .collect::<Vec<_>>();
        assert_eq!(transitions.len(), 2);
        let swept = match super::super::sweep::search(
            &stage.analysis.flow,
            &stage.analysis.merges,
            topology,
        ) {
            Ok(arrangement) => arrangement,
            Err(super::super::sweep::Refusal::Internal(reason)) => panic!("{reason}"),
            Err(super::super::sweep::Refusal::Impossible(reason)) => panic!("{}", reason.message),
        };
        for arrangement in [&stage.arrangement, &swept] {
            super::super::verify::arrangement(&stage.analysis.flow, topology, arrangement).unwrap();
            assert_eq!(
                arrangement.rank[&transitions[0]],
                arrangement.rank[&transitions[1]]
            );
            assert!(
                arrangement.column[&transitions[0]] > arrangement.column[&transitions[1]],
                "count is declared first but follows the right branch; finish follows the left"
            );
            let mut broken = arrangement.clone();
            *broken.rank.get_mut(&transitions[0]).unwrap() -= 1;
            assert!(verify(topology, &broken).is_err());
        }

        let function = syn::parse_quote! {
            fn diverging_branch(repeat: bool) {
                #[question("Repeat forever?")]
                let (again, finish) = |repeat| repeat;
                #[cycle("Repeat.")]
                |again| loop {
                    #[action("Work.")]
                    || ();
                    continue;
                };
                #[stage("Finish.")]
                |finish| { return; };
            }
        };
        let model = crate::build(&function).unwrap();
        let transition = model
            .topology
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::StageTransition)
            .unwrap();
        let row = model.arrangement.rank[&Vertex::Node(transition.id)];
        assert!(
            model
                .topology
                .cycles
                .iter()
                .all(|cycle| model.arrangement.rank[&Vertex::Junction(cycle.tail)] < row)
        );
        assert_eq!(
            model
                .topology
                .nodes
                .iter()
                .find(|node| node.id == NodeId::Start)
                .unwrap()
                .kind,
            NodeKind::Start
        );
    }
}
