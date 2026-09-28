//! Measures the composed stage diagram before any SVG is written.

use std::rc::Rc;

use kaalang_compiler::topology::{ExitId, NodeId, NodeKind, Topology, Vertex};
use kaalang_compiler::{Arrangement, BlockKind, SemanticModel};

use crate::captions;

use super::{CONNECTION_LABEL_HALO, Point, Scene, label_rect, layout_with_stage_rows, route};

const MARGIN: i32 = 24;
const PART_GAP: i32 = 36;
const RAIL_GAP: i32 = 36;

#[derive(Clone, Copy)]
pub(super) struct StageRows {
    pub(super) height: i32,
    pub(super) transition_y: i32,
}

pub(crate) struct StagedScene {
    pub(crate) direct_entry: Option<usize>,
    pub(crate) parts: Vec<(Scene, PartPlacement)>,
    pub(crate) connections: Vec<[Point; 2]>,
    pub(crate) return_route: Option<[Point; 4]>,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

pub(crate) struct PartPlacement {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) fn layout_staged(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
) -> Result<StagedScene, String> {
    let direct_entry = match model.analysis.flow.blocks.as_slice() {
        [transition, end] if end.kind == BlockKind::End => transition.transition_target,
        _ => None,
    };
    let models = std::iter::once(model)
        .chain(&model.stages)
        .collect::<Vec<_>>();
    let captions = models
        .iter()
        .enumerate()
        .map(|(part, model)| {
            Rc::new(captions::derive_stage(
                model,
                &models[0].analysis,
                part.checked_sub(1),
                start,
                return_type,
            ))
        })
        .collect::<Vec<_>>();
    let height = models
        .iter()
        .zip(&captions)
        .flat_map(|(model, captions)| {
            model
                .topology
                .nodes
                .iter()
                .filter(|node| matches!(node.kind, NodeKind::StageEntry | NodeKind::Transition))
                .map(|node| super::node_dimensions(node.kind, captions.label(node.id)).1)
        })
        .max()
        .expect("a staged flow has an entry");
    let rows = StageRows {
        height,
        transition_y: 0,
    };
    let mut parts = models
        .iter()
        .zip(&captions)
        .enumerate()
        .map(|(part, (model, captions))| {
            let parameters = if part == 0 { parameters } else { &[] };
            layout_with_stage_rows(model, captions, parameters, Some(rows))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if direct_entry.is_some() {
        keep_header(&mut parts[0]);
    }
    let placements = place(&parts, direct_entry);
    let transition_y = parts
        .iter()
        .zip(&placements)
        .map(|(scene, part)| {
            part.y
                + scene
                    .topology
                    .nodes
                    .iter()
                    .filter(|node| node.kind == NodeKind::Transition)
                    .map(|node| scene.node(node.id).y)
                    .max()
                    .unwrap_or(scene.height + RAIL_GAP + height / 2)
        })
        .max()
        .unwrap_or(0);
    for (index, scene) in parts.iter_mut().enumerate() {
        if scene
            .topology
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::Transition)
        {
            let parameters = if index == 0 { parameters } else { &[] };
            *scene = layout_with_stage_rows(
                models[index],
                &captions[index],
                parameters,
                Some(StageRows {
                    height,
                    transition_y: transition_y - placements[index].y,
                }),
            )?;
        }
    }
    let placements = place(&parts, direct_entry);
    let diagram = compose(parts, placements, direct_entry);
    verify(&diagram)?;
    Ok(diagram)
}

/// Empty preparation contributes only the function header. Composition connects
/// it directly to the entry selected by the input instead of drawing an address.
fn keep_header(scene: &mut Scene) {
    let start = Vertex::Node(NodeId::Start);
    let exit = ExitId::of(NodeId::Start);
    let mut topology = Topology::default();
    topology
        .nodes
        .push(scene.topology.node(NodeId::Start).clone());
    topology.exits.push(scene.topology.exit(exit).clone());
    topology.vertices.push(start);
    scene.topology = topology;
    scene.arrangement = Arrangement {
        rank: [(start, 0)].into(),
        ranks: 1,
        column: [(start, scene.arrangement.column[&start])].into(),
        exit_offset: [(exit, 0)].into(),
        ..Arrangement::default()
    };
    scene.nodes.retain(|node| node.id == NodeId::Start);
    scene.labels.retain(|label| label.owner == start);
    scene.connections.clear();
    scene.fit();
}

fn verify(diagram: &StagedScene) -> Result<(), String> {
    for (scene, placement) in &diagram.parts {
        let bounds = scene
            .nodes
            .iter()
            .map(Scene::bounds)
            .chain(scene.labels.iter().map(label_rect))
            .chain(scene.parameters.iter().map(Scene::parameter_bounds))
            .chain(scene.loop_regions.iter().map(super::LoopRegion::bounds));
        for (left, top, right, bottom) in bounds {
            let bounds = (
                placement.x + left,
                placement.y + top,
                placement.x + right,
                placement.y + bottom,
            );
            if diagram
                .connections
                .iter()
                .map(<[Point; 2]>::as_slice)
                .chain(diagram.return_route.iter().map(<[Point; 4]>::as_slice))
                .any(|rail| route::crosses(rail, bounds))
            {
                return Err("a silhouette rail crosses local content".to_owned());
            }
        }
    }
    Ok(())
}

fn compose(
    parts: Vec<Scene>,
    placements: Vec<PartPlacement>,
    direct_entry: Option<usize>,
) -> StagedScene {
    let first = usize::from(direct_entry.is_some());
    let first_x = placements[first].x + parts[first].node(NodeId::Start).x;
    let top = placements[1].y + Scene::bounds(parts[1].node(NodeId::Start)).1 - RAIL_GAP;
    let bottom = placements
        .iter()
        .zip(&parts)
        .map(|(part, scene)| part.y + scene.height)
        .max()
        .expect("a staged diagram contains preparation")
        + RAIL_GAP;
    let last = parts.len() - 1;
    let rail_right = placements[last].x + parts[last].node(NodeId::Start).x;
    let point = |x, y| Point { x, y };
    let mut connections = Vec::new();
    if direct_entry.is_some() {
        let start = parts[0].node(NodeId::Start);
        let x = placements[0].x + start.x;
        connections.push([
            point(x, placements[0].y + start.y + start.height / 2),
            point(x, top),
        ]);
    }
    let mut last_transition = None;
    for (index, (scene, part)) in parts.iter().zip(&placements).enumerate() {
        if index > 0 {
            let entry = scene.node(NodeId::Start);
            let x = part.x + entry.x;
            connections.push([point(x, top), point(x, part.y + entry.y - entry.height / 2)]);
        }
        for node in &scene.topology.nodes {
            if node.kind == NodeKind::Transition {
                let node = scene.node(node.id);
                let x = part.x + node.x;
                last_transition = Some(last_transition.unwrap_or(x).max(x));
                connections.push([
                    point(x, part.y + node.y + node.height / 2),
                    point(x, bottom),
                ]);
            }
        }
    }
    if first_x != rail_right {
        connections.push([point(first_x, top), point(rail_right, top)]);
    }
    let return_route = last_transition.map(|x| {
        [
            point(x, bottom),
            point(MARGIN, bottom),
            point(MARGIN, top),
            point(first_x, top),
        ]
    });
    let width = placements
        .iter()
        .zip(&parts)
        .map(|(part, scene)| part.x + scene.width)
        .max()
        .expect("a staged diagram contains preparation")
        + MARGIN;
    StagedScene {
        direct_entry,
        parts: parts.into_iter().zip(placements).collect(),
        connections,
        return_route,
        width,
        height: bottom + MARGIN,
    }
}

fn place(parts: &[Scene], direct_entry: Option<usize>) -> Vec<PartPlacement> {
    let mut placements = Vec::with_capacity(parts.len());
    let mut x = MARGIN + RAIL_GAP;
    let start = parts[0].node(NodeId::Start);
    let start_height = parts[0]
        .parameters
        .as_ref()
        .map_or(start.height, |panel| panel.height.max(start.height));
    let rail_y = parts[0]
        .labels
        .iter()
        .filter(|label| label.owner == Vertex::Node(NodeId::Start))
        .map(|label| label_rect(label).3 + CONNECTION_LABEL_HALO)
        .fold(start.y + start_height / 2 + RAIL_GAP, i32::max);
    let stage_y = MARGIN + rail_y + RAIL_GAP;
    for (part, scene) in parts.iter().enumerate() {
        placements.push(PartPlacement {
            x,
            y: if part == 0 {
                MARGIN
            } else {
                stage_y - Scene::bounds(scene.node(NodeId::Start)).1
            },
        });
        if part > 0 || direct_entry.is_none() {
            x += scene.body_size().0 + PART_GAP;
        }
    }
    if let Some(entry) = direct_entry {
        placements[0].x =
            placements[entry + 1].x + parts[entry + 1].node(NodeId::Start).x - start.x;
    }
    placements
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn drawn(source: &str, name: &str, return_type: &str) -> StagedScene {
        let function = syn::parse_str(source).unwrap();
        let model = kaalang_compiler::build(&function).unwrap();
        layout_staged(&model, name, &[], return_type).unwrap()
    }

    #[test]
    fn direct_inputs_share_the_receiving_stage_column() {
        for (source, entry) in [
            (
                include_str!("../../../kaalang/tests/gallery/sorting/quick_sort.rs"),
                0,
            ),
            (
                r#"
                    #[kaalang]
                    fn example(go: u8) -> u8 {
                        #[stage("Forward the value.")]
                        let finish = |forward| { #[action("Use the value.")] let finish = |forward| forward; };
                        #[stage("Begin.")]
                        let forward = |go| {
                            #[action("Use the input.")] let forward = |go| go;
                        };
                        #[stage("Finish.")]
                        |finish| { |finish| return finish; };
                    }
                "#,
                1,
            ),
            (
                r#"
                    #[kaalang]
                    fn example(go: (), a_very_long_parameter_name: u8,
                        another_very_long_parameter_name: u8,
                        one_more_very_long_parameter_name: u8) -> u8 {
                        #[stage("Finish.")]
                        |go| {
                            #[action("Add the inputs.")]
                            let sum = |a_very_long_parameter_name,
                                another_very_long_parameter_name,
                                one_more_very_long_parameter_name|
                                a_very_long_parameter_name + another_very_long_parameter_name
                                    + one_more_very_long_parameter_name;
                            |sum| return sum;
                        };
                    }
                "#,
                0,
            ),
            (
                r#"
                    #[kaalang]
                    fn example(go: ()) -> ! {
                        #[stage("Repeat forever.")]
                        |go| { #[cycle("Repeat.")] { continue; }; };
                    }
                "#,
                0,
            ),
        ] {
            let file = syn::parse_file(source).unwrap();
            let function = kaalang_compiler::flows(&file.items).remove(0);
            let parameters = super::super::parameter_text(source, &function.sig);
            for collapsed in [false, true] {
                let mut model = kaalang_compiler::build_with_options(&function, collapsed).unwrap();
                kaalang_render::compact_arrangement(&mut model);
                for stage in &mut model.stages {
                    kaalang_render::compact_arrangement(stage);
                }
                let diagram = layout_staged(&model, "example", &parameters, "u8").unwrap();
                assert_eq!(diagram.direct_entry, Some(entry));
                assert_eq!(diagram.parts[1].1.x, MARGIN + RAIL_GAP);
                let (header, placement) = &diagram.parts[0];
                let (stage, stage_placement) = &diagram.parts[entry + 1];
                assert_eq!(header.nodes.len(), 1);
                assert_eq!(
                    placement.x + header.node(NodeId::Start).x,
                    stage_placement.x + stage.node(NodeId::Start).x
                );
                let panel = header.parameters.as_ref().unwrap();
                assert!(
                    placement.y + Scene::parameter_bounds(panel).3
                        < stage_placement.y + Scene::bounds(stage.node(NodeId::Start)).1 - RAIL_GAP
                );
                assert!(placement.x + Scene::parameter_bounds(panel).2 < diagram.width);
                assert!(
                    diagram.parts[1..]
                        .windows(2)
                        .all(|pair| pair[0].1.x < pair[1].1.x)
                );
                for (local, _) in &diagram.parts {
                    assert_eq!(super::super::correspondence(local), None);
                }
                assert_eq!(
                    diagram.return_route.is_some(),
                    diagram.parts[1..].iter().any(|(part, _)| part
                        .topology
                        .nodes
                        .iter()
                        .any(|node| node.kind == NodeKind::Transition))
                );
                let svg = crate::svg::serialize_staged(&diagram, &model.analysis, "example");
                assert!(svg.contains(&format!(
                    "The flow starts directly at stage {}.",
                    model.analysis.stages[entry].entry_alias
                )));
            }
        }
    }

    #[test]
    fn silhouette_rail_clears_multiline_start_handovers() {
        let source = include_str!("../../../kaalang/tests/stage/behavior/sum_inputs.rs");
        let file = syn::parse_file(source).unwrap();
        let function = kaalang_compiler::flows(&file.items).remove(0);
        let mut model = kaalang_compiler::build(&function).unwrap();
        kaalang_render::compact_arrangement(&mut model);
        for stage in &mut model.stages {
            kaalang_render::compact_arrangement(stage);
        }
        let parameters = super::super::parameter_text(source, &function.sig);
        let diagram = layout_staged(&model, "sum_inputs", &parameters, "u32").unwrap();
        let (preparation, placement) = &diagram.parts[0];
        let rail_y = diagram.return_route.as_ref().unwrap()[3].y;
        let handover = preparation
            .labels
            .iter()
            .find(|label| label.owner == Vertex::Node(NodeId::Start))
            .unwrap();
        assert!(handover.lines.len() >= 4);
        assert!(placement.y + super::super::label_rect(handover).3 < rail_y);
    }

    #[test]
    fn silhouette_aligns_entries_and_addresses_and_connects_the_outer_contour() {
        let source = include_str!("../../../kaalang/tests/gallery/kmp_search/mod.rs");
        let file = syn::parse_file(source).unwrap();
        let function = kaalang_compiler::flows(&file.items).remove(0);
        for collapsed in [false, true] {
            let mut model = kaalang_compiler::build_with_options(&function, collapsed).unwrap();
            kaalang_render::compact_arrangement(&mut model);
            for stage in &mut model.stages {
                kaalang_render::compact_arrangement(stage);
            }
            let diagram = layout_staged(
                &model,
                "kmp_search",
                &["text: &[u8]".to_owned(), "pattern: &[u8]".to_owned()],
                "Option<usize>",
            )
            .unwrap();
            let return_route = diagram.return_route.as_ref().unwrap();
            let mut entries = BTreeSet::new();
            let mut addresses = BTreeSet::new();
            let rails = diagram
                .connections
                .iter()
                .map(<[Point; 2]>::as_slice)
                .chain(std::iter::once(return_route.as_slice()))
                .collect::<Vec<_>>();
            for (scene, part) in &diagram.parts {
                assert_eq!(super::super::correspondence(scene), None);
                for node in &scene.nodes {
                    let kind = scene.topology.node(node.id).kind;
                    let x = part.x + node.x;
                    let y = part.y + node.y;
                    if kind == NodeKind::StageEntry {
                        entries.insert(y);
                        assert!(diagram.connections.iter().any(|line| line[1]
                            == Point {
                                x,
                                y: y - node.height / 2
                            }));
                    } else if kind == NodeKind::Transition {
                        addresses.insert(y);
                        assert!(diagram.connections.iter().any(|line| line[0]
                            == Point {
                                x,
                                y: y + node.height / 2
                            }
                            && line[1].y == return_route[0].y));
                    }
                    for rail in &rails {
                        assert!(!super::super::route::crosses(
                            rail,
                            (
                                x - node.width / 2,
                                y - node.height / 2,
                                x + node.width / 2,
                                y + node.height / 2
                            )
                        ));
                    }
                }
            }
            assert_eq!(entries.len(), 1);
            assert_eq!(addresses.len(), 1);
            let (prefix, prefix_placement) = &diagram.parts[0];
            let first_computation = prefix
                .nodes
                .iter()
                .find(|node| {
                    matches!(
                        prefix.topology.node(node.id).kind,
                        NodeKind::Action | NodeKind::Call
                    )
                })
                .unwrap();
            let body_top = prefix_placement.y + Scene::bounds(first_computation).1;
            for (stage, placement) in &diagram.parts[1..] {
                assert_eq!(
                    placement.y + Scene::bounds(stage.node(NodeId::Start)).1,
                    body_top
                );
            }
            assert_eq!(prefix.topology.node(NodeId::Start).kind, NodeKind::Start);
            assert_eq!(prefix.captions.label(NodeId::Start).as_ref(), "kmp_search");
            assert_eq!(return_route[1].x, MARGIN);
            assert_eq!(return_route[2].x, MARGIN);
            assert!(diagram.parts.iter().all(|(_, part)| part.x > MARGIN));
            assert_eq!(
                return_route[3].x,
                prefix_placement.x + prefix.node(NodeId::Start).x
            );
            assert!(return_route[3].y < *entries.first().unwrap());
            assert!(prefix_placement.y + prefix.node(NodeId::Start).y < return_route[3].y);
            assert!(prefix.parameters.is_some());
            let without_parameters =
                layout_staged(&model, "kmp_search", &[], "Option<usize>").unwrap();
            assert_eq!(
                diagram.parts[1].1.x, without_parameters.parts[1].1.x,
                "the parameter panel must not reserve an extra body column"
            );
        }
    }

    #[test]
    fn initial_addresses_join_the_contour_without_stretching_the_terminal_stage() {
        let staged = drawn(
            r#"
                #[kaalang]
                fn long_signal() {
                    #[action("Begin.")]
                    let this_is_a_very_very_very_long_signal_name = || {};
                    #[stage("Finish.")]
                    |this_is_a_very_very_very_long_signal_name| { return; };
                }
            "#,
            "long_signal",
            "()",
        );
        let (preparation, first) = &staged.parts[0];
        let (terminal, second) = &staged.parts[1];
        assert!(second.x > first.x + preparation.width);
        assert!(staged.return_route.is_some());
        let end_id = terminal
            .topology
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::End)
            .unwrap()
            .id;
        let end = terminal.node(end_id);
        let address = preparation
            .nodes
            .iter()
            .find(|node| preparation.topology.node(node.id).kind == NodeKind::Transition)
            .unwrap();
        assert!(second.y + end.y + end.height / 2 < first.y + address.y);
        assert_eq!(super::super::correspondence(terminal), None);
    }
}
