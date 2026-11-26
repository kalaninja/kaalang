//! What a staged flow draws of an empty preparation.

use kaalang_compiler::{
    Arrangement, SemanticModel,
    topology::{ExitId, NodeId, Topology, Vertex},
};

/// Keeps only the start of a staged flow's empty preparation, where its
/// arrangement placed it, and returns the stage the preparation enters. Its
/// function header then leads straight to that stage's entry, without the
/// transition node.
#[must_use]
pub fn keep_header(model: &mut SemanticModel) -> Option<usize> {
    // An empty preparation holds only its transition and its end.
    let [transition, _] = model.analysis.flow.blocks.as_slice() else {
        return None;
    };
    if model.stages.is_empty() {
        return None;
    }
    let entry = transition.transition_target?;
    let start = Vertex::Node(NodeId::Start);
    let exit = ExitId::of(NodeId::Start);
    let mut topology = Topology::default();
    topology
        .nodes
        .push(model.topology.node(NodeId::Start).clone());
    topology.exits.push(model.topology.exit(exit).clone());
    topology.vertices.push(start);
    model.topology = topology;
    model.arrangement = Arrangement {
        rank: [(start, 0)].into(),
        ranks: 1,
        column: [(start, model.arrangement.column[&start])].into(),
        exit_offset: [(exit, 0)].into(),
        ..Arrangement::default()
    };
    Some(entry)
}
