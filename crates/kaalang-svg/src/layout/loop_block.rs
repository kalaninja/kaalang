//! Measures and verifies expanded cycle boundaries around their arranged bodies.

use kaalang_model::topology::{Destination, NodeId, Source, Vertex};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    CYCLE_CAPTION_FONT, LoopRegion, MARGIN, NODE_LABEL_WIDTH, NODE_WIDTH, Point, Scene,
    block_dimensions, label::label_rect, text,
};

const VERTICAL_PADDING: i32 = 35;

/// Nested boundaries ending on one row stack their padding. The usual row gap
/// holds one boundary; reserve the additional layers before the next row.
pub(super) fn bottom_padding(scene: &Scene) -> Vec<i32> {
    let last_rows = scene
        .region_bodies
        .iter()
        .zip(&scene.topology.loop_boundaries)
        .map(|(body, boundary)| {
            body.iter()
                .copied()
                .chain(boundary.result.map(Vertex::from))
                .map(|vertex| scene.rank(vertex))
                .max()
                .expect("every cycle body includes its entry")
        })
        .collect::<Vec<_>>();
    let mut padding = vec![0; scene.arrangement.ranks];
    for (boundary, &row) in scene.topology.loop_boundaries.iter().zip(&last_rows) {
        let layers = scene
            .topology
            .loop_boundaries
            .iter()
            .zip(&last_rows)
            .filter(|(outer, last)| {
                **last == row && (outer.header..outer.end).contains(&boundary.header)
            })
            .count();
        padding[row] = padding[row].max((layers as i32 - 1) * VERTICAL_PADDING);
    }
    padding
}

pub(super) fn dimensions(label: &str) -> (i32, i32, Vec<String>) {
    block_dimensions(label, NODE_WIDTH, NODE_LABEL_WIDTH - 28, 64)
}

#[allow(clippy::too_many_lines)]
pub(super) fn regions(scene: &Scene) -> Vec<LoopRegion> {
    let point = |vertex| {
        let junction = match vertex {
            Vertex::Node(node) => return Some(scene.top_anchor(node)),
            Vertex::Junction(junction) => junction,
        };
        scene.connections.iter().find_map(|edge| {
            if edge.source == Source::Junction(junction) {
                edge.points.first().copied()
            } else if edge.destination == Destination::Junction(junction) {
                edge.points.last().copied()
            } else {
                None
            }
        })
    };
    let exit_point = |source| match source {
        Source::Exit(exit) => Some(scene.exit_anchor(exit)),
        Source::Junction(junction) => point(Vertex::Junction(junction)),
    };
    let mut regions = Vec::<(usize, LoopRegion)>::new();
    for (index, boundary) in scene.topology.loop_boundaries.iter().enumerate().rev() {
        let body = &scene.region_bodies[index];
        let owns = |id: NodeId| body.contains(&Vertex::Node(id));
        let owns_vertex = |vertex| {
            body.contains(&vertex)
                || vertex == boundary.entry
                || Some(vertex) == boundary.result.map(Vertex::from)
        };
        let mut boxes = scene
            .nodes
            .iter()
            .filter(|node| owns(node.id))
            .map(Scene::bounds)
            .collect::<Vec<_>>();
        boxes.extend(
            regions
                .iter()
                .filter(|(header, _)| (boundary.header + 1..boundary.end).contains(header))
                .map(|(_, region)| (region.left, region.top, region.right, region.bottom)),
        );
        let body_bottom = boxes.iter().map(|bounds| bounds.3).max();
        boxes.extend(
            scene
                .labels
                .iter()
                .filter(|label| owns_vertex(label.owner))
                .map(label_rect),
        );
        let mut points = [point(boundary.entry), boundary.result.and_then(exit_point)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        points.extend(
            scene
                .connections
                .iter()
                .filter(|edge| {
                    owns_vertex(Vertex::from(edge.source)) && owns_vertex(edge.destination)
                })
                .flat_map(|edge| edge.points.iter().copied()),
        );
        if let Some(loop_) = scene
            .topology
            .loops
            .iter()
            .find(|loop_| loop_.header == boundary.header)
            && let Some(back) = scene
                .connections
                .iter()
                .find(|edge| edge.source == Source::Junction(loop_.tail))
        {
            points.extend(back.points.iter().copied());
        }
        let left = boxes
            .iter()
            .map(|bounds| bounds.0)
            .chain(points.iter().map(|point| point.x))
            .min()
            .unwrap_or(MARGIN)
            - 28;
        let right = boxes
            .iter()
            .map(|bounds| bounds.2)
            .chain(points.iter().map(|point| point.x))
            .max()
            .unwrap_or(left + NODE_WIDTH)
            + 28;
        let entry = point(boundary.entry).unwrap_or(Point {
            x: i32::midpoint(left, right),
            y: MARGIN,
        });
        let description = scene.captions.label(NodeId::Block(boundary.header));
        // Keep the horizontal entry and result rails inside the rectangle.
        // Putting either junction on its edge makes the dashed boundary and
        // the connection share a visible run.
        let top = (entry.y - VERTICAL_PADDING)
            .min(boxes.iter().map(|bounds| bounds.1).min().unwrap_or(entry.y));
        let caption_left = scene
            .connections
            .iter()
            .flat_map(|edge| edge.points.windows(2))
            .filter(|segment| {
                super::route::enters(segment[0], segment[1], (left, top, right, entry.y))
            })
            .map(|segment| segment[0].x.max(segment[1].x))
            .chain(
                scene
                    .labels
                    .iter()
                    .map(label_rect)
                    .filter(|bounds| {
                        bounds.1 < entry.y && bounds.3 > top && bounds.0 < right && bounds.2 > left
                    })
                    .map(|bounds| bounds.2),
            )
            .max()
            .unwrap_or(left);
        let body_bottom = body_bottom
            .into_iter()
            .chain(points.iter().map(|point| point.y))
            .max()
            .unwrap_or(entry.y);
        let bottom = (body_bottom + VERTICAL_PADDING).max(
            boxes
                .iter()
                .map(|bounds| bounds.3)
                .max()
                .unwrap_or(body_bottom),
        );
        regions.push((
            boundary.header,
            LoopRegion {
                left,
                top,
                right,
                bottom,
                description: description.to_owned(),
                caption: caption(description, right - caption_left - 24),
                inputs: scene.captions.loop_inputs(boundary.header).to_owned(),
                outputs: scene.captions.loop_outputs(boundary.header).to_owned(),
            },
        ));
    }
    regions.reverse();
    regions.into_iter().map(|(_, region)| region).collect()
}

/// Fits the optional caption beside the incoming route in the existing top
/// padding. The complete description remains available through the SVG title.
fn caption(description: &str, width: i32) -> Vec<String> {
    let ellipsis = text::text_width("…", CYCLE_CAPTION_FONT);
    if width < ellipsis {
        return Vec::new();
    }
    let mut lines = text::wrap_text(description, width, CYCLE_CAPTION_FONT);
    for index in 0..lines.len().min(2) {
        if (index == 1 && lines.len() > 2)
            || text::text_width(&lines[index], CYCLE_CAPTION_FONT) > width
        {
            lines.truncate(index + 1);
            let last = &mut lines[index];
            while text::text_width(last, CYCLE_CAPTION_FONT) + ellipsis > width {
                let (end, _) = last
                    .grapheme_indices(true)
                    .next_back()
                    .expect("an overwide caption still contains text");
                last.truncate(end);
            }
            last.push('…');
            break;
        }
    }
    lines
}

#[allow(clippy::too_many_lines)]
pub(super) fn verify(scene: &Scene) -> Option<String> {
    if scene.topology.loop_boundaries.len() != scene.loop_regions.len() {
        return Some("a cycle boundary is missing from the diagram".to_owned());
    }
    for (index, (boundary, region)) in scene
        .topology
        .loop_boundaries
        .iter()
        .zip(&scene.loop_regions)
        .enumerate()
    {
        if region.left < 0
            || region.top < 0
            || region.left >= region.right
            || region.top >= region.bottom
            || region.right > scene.width
            || region.bottom > scene.height
        {
            return Some("a cycle boundary lies outside the diagram".to_owned());
        }
        let body = &scene.region_bodies[index];
        let owns = |id: NodeId| body.contains(&Vertex::Node(id));
        let owns_vertex = |vertex| {
            body.contains(&vertex)
                || vertex == boundary.entry
                || Some(vertex) == boundary.result.map(Vertex::from)
        };
        for node in &scene.nodes {
            let (left, top, right, bottom) = Scene::bounds(node);
            let inside = left >= region.left
                && top >= region.top
                && right <= region.right
                && bottom <= region.bottom;
            let overlaps = left < region.right
                && right > region.left
                && top < region.bottom
                && bottom > region.top;
            if owns(node.id) && !inside {
                return Some(format!(
                    "cycle {} does not contain owned node {:?}",
                    boundary.header, node.id
                ));
            }
            if !owns(node.id) && overlaps {
                return Some(format!(
                    "cycle {} {:?} overlaps external node {:?} {:?}",
                    boundary.header,
                    (region.left, region.top, region.right, region.bottom),
                    node.id,
                    (left, top, right, bottom)
                ));
            }
        }
        for label in &scene.labels {
            let (left, top, right, bottom) = label_rect(label);
            let inside = left >= region.left
                && top >= region.top
                && right <= region.right
                && bottom <= region.bottom;
            let overlaps = left < region.right
                && right > region.left
                && top < region.bottom
                && bottom > region.top;
            if owns_vertex(label.owner) && !inside {
                return Some(format!(
                    "cycle {} does not contain a label owned by {:?}",
                    boundary.header, label.owner
                ));
            }
            if !owns_vertex(label.owner) && overlaps {
                return Some(format!(
                    "cycle {} overlaps a label owned by {:?}",
                    boundary.header, label.owner
                ));
            }
        }
        for edge in &scene.connections {
            if edge
                .points
                .windows(2)
                .any(|segment| overlaps_boundary(segment, region))
            {
                return Some(format!(
                    "cycle {} boundary overlaps route {:?} -> {:?}",
                    boundary.header, edge.source, edge.destination
                ));
            }
            let source_owned = owns_vertex(Vertex::from(edge.source));
            let destination_owned = owns_vertex(edge.destination);
            let crosses_interface =
                edge.destination == boundary.entry || boundary.result == Some(edge.source);
            if source_owned
                && destination_owned
                && edge.points.iter().any(|point| {
                    point.x < region.left
                        || point.x > region.right
                        || point.y < region.top
                        || point.y > region.bottom
                })
            {
                return Some(format!(
                    "cycle {} {:?} does not contain internal route {:?} -> {:?}: {:?}",
                    boundary.header,
                    (region.left, region.top, region.right, region.bottom),
                    edge.source,
                    edge.destination,
                    edge.points
                ));
            }
            if !(crosses_interface || source_owned && destination_owned)
                && edge.points.windows(2).any(|segment| {
                    super::route::enters(
                        segment[0],
                        segment[1],
                        (region.left, region.top, region.right, region.bottom),
                    )
                })
            {
                return Some(format!(
                    "cycle {} is crossed by external route {:?} -> {:?}",
                    boundary.header, edge.source, edge.destination
                ));
            }
        }
        for (nested, nested_region) in scene
            .topology
            .loop_boundaries
            .iter()
            .zip(&scene.loop_regions)
        {
            if nested.header == boundary.header {
                continue;
            }
            let inside = nested_region.left >= region.left
                && nested_region.top >= region.top
                && nested_region.right <= region.right
                && nested_region.bottom <= region.bottom;
            let overlaps = nested_region.left < region.right
                && nested_region.right > region.left
                && nested_region.top < region.bottom
                && nested_region.bottom > region.top;
            let nested_owned = (boundary.header + 1..boundary.end).contains(&nested.header);
            let enclosing = (nested.header + 1..nested.end).contains(&boundary.header);
            if (nested_owned && !inside) || (!nested_owned && !enclosing && overlaps) {
                return Some(format!(
                    "cycle {} {:?} and cycle {} {:?} overlap or interleave (owned: {nested_owned})",
                    boundary.header,
                    (region.left, region.top, region.right, region.bottom),
                    nested.header,
                    (
                        nested_region.left,
                        nested_region.top,
                        nested_region.right,
                        nested_region.bottom
                    )
                ));
            }
            if nested_owned
                && scene.connections.iter().any(|edge| {
                    edge.destination == boundary.entry
                        && scene.is_back_edge(edge)
                        && edge.points.windows(2).any(|segment| {
                            super::route::enters(
                                segment[0],
                                segment[1],
                                (
                                    nested_region.left - super::LANE,
                                    nested_region.top - super::LANE,
                                    nested_region.right + super::LANE,
                                    nested_region.bottom + super::LANE,
                                ),
                            )
                        })
                })
            {
                return Some(format!(
                    "cycle {} back edge runs too close to cycle {}",
                    boundary.header, nested.header
                ));
            }
        }
    }
    None
}

fn overlaps_boundary(segment: &[Point], region: &LoopRegion) -> bool {
    let [a, b] = segment else { return false };
    if a.y == b.y {
        (a.y == region.top || a.y == region.bottom)
            && a.x.min(b.x).max(region.left) < a.x.max(b.x).min(region.right)
    } else {
        (a.x == region.left || a.x == region.right)
            && a.y.min(b.y).max(region.top) < a.y.max(b.y).min(region.bottom)
    }
}
