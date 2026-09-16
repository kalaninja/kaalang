//! Keeps end below the rest of the diagram, including iteration back edges.

use super::Scene;
use kaalang_compiler::topology::NodeKind;

/// Nothing may end below the final block.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    let end = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)?;
    let top = Scene::bounds(scene.node(end.id)).1;
    let misplaced_node = scene
        .nodes
        .iter()
        .any(|node| node.id != end.id && Scene::bounds(node).3 >= top);
    // A back edge beside end may align with its top edge; the block itself still
    // sits below it. No connection may descend alongside the block's body.
    let misplaced_route = scene
        .connections
        .iter()
        .any(|edge| edge.points.iter().any(|point| point.y > top));
    (misplaced_node || misplaced_route)
        .then(|| "end must be below every other node and iteration back edge".to_owned())
}
