//! Lays out dependency graphs whose independent branch dispatches cannot be
//! represented by the structured plan's branch tree.

use std::collections::{BTreeSet, HashMap};

use kaalang_model::{BlockKind, ExecutionPlan, ProducerId, SemanticModel};

use super::{
    Builder, Incoming, MARGIN, NodeId, NodeKind, Origin, Point, Scene, Side, compact_points,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Connection {
    from: NodeId,
    branch: Option<usize>,
    to: NodeId,
}

/// A guarded schedule has no branch tree from which to derive coordinates.
pub(super) fn is_graph(plan: &ExecutionPlan) -> bool {
    match plan {
        ExecutionPlan::End { body, .. } | ExecutionPlan::Action { next: body, .. } => {
            is_graph(body)
        }
        ExecutionPlan::Guarded { .. } => true,
        ExecutionPlan::Question { branches, join, .. } => {
            branches.iter().any(|branch| is_graph(&branch.plan))
                || join.as_ref().is_some_and(|join| is_graph(&join.next))
        }
        ExecutionPlan::Choice {
            branches, joins, ..
        } => {
            branches.iter().any(|branch| is_graph(&branch.plan))
                || joins.iter().any(|join| is_graph(&join.next))
        }
        ExecutionPlan::EndArrival { .. } | ExecutionPlan::Yield { .. } => false,
    }
}

/// Union of the direct dependencies of each execution, including structural
/// completion and ordering before implicit wire merges. Reducing each
/// execution separately preserves a connection that is direct in one execution
/// and redundant in another.
fn connections(graph: &SemanticModel) -> Vec<Connection> {
    let nodes = nodes(graph);
    let indexes = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (*node, i))
        .collect::<HashMap<_, _>>();
    let end = NodeId::Block(graph.flow.blocks.len() - 1);
    let mut all = BTreeSet::new();
    for execution in &graph.executions {
        let selected = |block| {
            execution
                .branches
                .iter()
                .find(|branch| branch.block == block)
                .map(|branch| branch.branch)
        };
        let exit = |block: usize| match graph.flow.blocks[block].kind {
            BlockKind::Action => (NodeId::Block(block), None),
            BlockKind::Question => (
                NodeId::Block(block),
                Some(selected(block).expect("an executed question selects one branch")),
            ),
            BlockKind::Choice => {
                let branch = selected(block).expect("an executed choice selects one case");
                (
                    NodeId::Case {
                        choice: block,
                        branch,
                    },
                    Some(branch),
                )
            }
            BlockKind::End => unreachable!("end has no outgoing connection"),
        };
        let mut edges = BTreeSet::new();
        for dependency in &execution.dependencies {
            let (from, branch) = match dependency.producer {
                ProducerId::FlowInput(_) => (NodeId::Start, None),
                ProducerId::BlockOutput { block, .. } => exit(block),
            };
            edges.insert(Connection {
                from,
                branch,
                to: NodeId::Block(dependency.capture.block),
            });
        }
        for merge in &graph.merges {
            for dependency in execution.dependencies.iter().filter(|dependency| {
                let capture = dependency.capture;
                graph.flow.blocks[capture.block].inputs[capture.input].ident == merge.wire
            }) {
                for &block in merge
                    .before
                    .iter()
                    .filter(|&&block| execution.participates(block))
                {
                    let (from, branch) = exit(block);
                    edges.insert(Connection {
                        from,
                        branch,
                        to: NodeId::Block(dependency.capture.block),
                    });
                }
            }
        }
        for &block in &execution.blocks {
            let (from, branch) = exit(block);
            if matches!(from, NodeId::Case { .. }) {
                edges.insert(Connection {
                    from: NodeId::Block(block),
                    branch: None,
                    to: from,
                });
            }
            edges.insert(Connection {
                from,
                branch,
                to: end,
            });
        }
        let mut precedes = vec![vec![false; nodes.len()]; nodes.len()];
        for edge in &edges {
            precedes[indexes[&edge.from]][indexes[&edge.to]] = true;
        }
        for middle in 0..nodes.len() {
            for from in 0..nodes.len() {
                for to in 0..nodes.len() {
                    precedes[from][to] |= precedes[from][middle] && precedes[middle][to];
                }
            }
        }
        all.extend(edges.into_iter().filter(|edge| {
            let from = indexes[&edge.from];
            let to = indexes[&edge.to];
            !(0..nodes.len()).any(|middle| precedes[from][middle] && precedes[middle][to])
        }));
    }
    all.into_iter().collect()
}

fn nodes(graph: &SemanticModel) -> Vec<NodeId> {
    let mut nodes = vec![NodeId::Start];
    for (index, block) in graph.flow.blocks.iter().enumerate() {
        nodes.push(NodeId::Block(index));
        nodes.extend(
            (0..block.case_descriptions.len()).map(|branch| NodeId::Case {
                choice: index,
                branch,
            }),
        );
    }
    nodes
}

fn rows(nodes: &[NodeId], connections: &[Connection]) -> HashMap<NodeId, usize> {
    let mut rows = HashMap::new();
    // A wire merge can order later-authored local work before a consumer.
    // ponytail: rescanning every connection per pending node is O(nodes² ×
    // connections); switch to Kahn with in-degree counts, as `plan::order`
    // does, if diagrams outgrow a few dozen nodes.
    let mut pending = nodes.iter().copied().collect::<BTreeSet<_>>();
    while !pending.is_empty() {
        let node = *pending
            .iter()
            .find(|&&node| {
                connections
                    .iter()
                    .filter(|edge| edge.to == node)
                    .all(|edge| rows.contains_key(&edge.from))
            })
            .expect("validated wire merges leave an acyclic visual graph");
        pending.remove(&node);
        let row = connections
            .iter()
            .filter(|edge| edge.to == node)
            .map(|edge| rows[&edge.from] + 1)
            .max()
            .unwrap_or(usize::from(node != NodeId::Start));
        rows.insert(node, row);
    }
    rows
}

fn place_nodes(
    builder: &mut Builder<'_>,
    nodes: &[NodeId],
    connections: &[Connection],
    rows: &HashMap<NodeId, usize>,
    signature: &str,
) -> HashMap<NodeId, usize> {
    let mut ordered_nodes = nodes.iter().copied().enumerate().collect::<Vec<_>>();
    ordered_nodes.sort_by_key(|(index, id)| (rows[id], *index));
    let mut columns = HashMap::new();
    let mut occupied = HashMap::<usize, usize>::new();
    for (_, id) in ordered_nodes {
        let row = rows[&id];
        let preferred = connections.iter().filter(|edge| edge.to == id).map(|edge| {
            columns[&edge.from] + if matches!(edge.from, NodeId::Block(index) if builder.graph.flow.blocks[index].kind == BlockKind::Question) { edge.branch.unwrap_or(0) } else { 0 }
        }).min().unwrap_or(0);
        let (column, span) = match id {
            NodeId::Case { choice, branch } => (columns[&NodeId::Block(choice)] + branch, 1),
            NodeId::Block(index) => {
                let block = &builder.graph.flow.blocks[index];
                let span = match block.kind {
                    BlockKind::Question | BlockKind::Choice => block.outputs.len(),
                    _ => 1,
                };
                (preferred.max(*occupied.get(&row).unwrap_or(&0)), span)
            }
            NodeId::Start => (0, 1),
        };
        occupied
            .entry(row)
            .and_modify(|next| *next = (*next).max(column + span))
            .or_insert(column + span);
        columns.insert(id, column);
        match id {
            NodeId::Start => {
                builder.add_node(id, NodeKind::Start, signature.to_owned(), column, 0);
            }
            NodeId::Block(index) => {
                builder.add_authored_node(index, column, 0);
                if builder.graph.flow.blocks[index].kind == BlockKind::Choice {
                    occupied
                        .entry(row + 1)
                        .and_modify(|next| *next = (*next).max(column + span))
                        .or_insert(column + span);
                }
            }
            NodeId::Case { choice, branch } => {
                builder.add_node(
                    id,
                    NodeKind::Case,
                    builder.graph.flow.blocks[choice].case_descriptions[branch].clone(),
                    column,
                    0,
                );
            }
        }
    }
    let mut top = MARGIN;
    for row in 0..=rows.values().copied().max().unwrap_or(0) {
        let height = builder
            .scene
            .nodes
            .iter()
            .filter(|node| rows[&node.id] == row)
            .map(|node| node.height)
            .max()
            .unwrap_or(0);
        for node in &mut builder.scene.nodes {
            if rows[&node.id] == row {
                node.y = top + node.height / 2;
            }
        }
        top += height + builder.vertical_gap + 96;
    }

    columns
}

pub(super) fn layout(mut builder: Builder<'_>, signature: &str) -> Scene {
    let connections = connections(builder.graph);
    let nodes = nodes(builder.graph);
    let rows = rows(&nodes, &connections);
    let columns = place_nodes(&mut builder, &nodes, &connections, &rows, signature);
    let mut ordered = connections;
    ordered.sort_by_key(|edge| {
        (
            edge.to == *nodes.last().expect("a flow has end"),
            rows[&edge.to] - rows[&edge.from],
            edge.from,
            edge.branch,
            edge.to,
        )
    });
    let mut routed = Vec::new();
    for edge in ordered {
        let origin = match edge.from {
            NodeId::Block(index)
                if builder.graph.flow.blocks[index].kind == BlockKind::Question
                    && edge.branch == Some(1) =>
            {
                Origin::right(edge.from)
            }
            _ => Origin::bottom(edge.from),
        };
        let incoming = Incoming {
            origin,
            branch: edge.branch,
            skewer: columns[&edge.from],
        };
        let points = route(&builder, edge, origin, &routed);
        if matches!(edge.to, NodeId::Case { .. }) {
            builder.connect_points(edge.from, edge.to, Vec::new(), Vec::new(), points.clone());
        } else {
            builder.connect(incoming, edge.to, points.clone());
        }
        routed.push((edge, points));
    }
    builder.place_labels();
    builder.fit_scene();
    builder.scene
}

fn route(
    builder: &Builder<'_>,
    edge: Connection,
    origin: Origin,
    routed: &[(Connection, Vec<Point>)],
) -> Vec<Point> {
    // ponytail: row-gap lanes avoid nodes and prefer fewer crossings; plan 5
    // replaces this bounded router with crossing-free topology layout.
    const CLEARANCE: i32 = 48;
    let start = builder.anchor(origin);
    let end = builder.top_anchor(edge.to);
    let exit = match origin.side {
        Side::Bottom => start,
        Side::Right => Point {
            x: start.x + CLEARANCE,
            y: start.y,
        },
    };
    let source_top = builder.top_anchor(edge.from).y;
    let departure = builder
        .scene
        .nodes
        .iter()
        .filter(|node| node.y - node.height / 2 == source_top)
        .map(|node| node.y + node.height / 2)
        .max()
        .unwrap_or(start.y)
        + CLEARANCE;
    let arrival = end.y - CLEARANCE;
    let outside = builder
        .scene
        .nodes
        .iter()
        .map(|node| node.x + node.width / 2)
        .max()
        .unwrap_or(0)
        + CLEARANCE;
    let lanes = [exit.x, end.x, outside].into_iter().chain(
        builder
            .scene
            .nodes
            .iter()
            .map(|node| node.x + node.width / 2 + CLEARANCE),
    );
    lanes
        .chain((1..=routed.len()).map(|lane| outside + lane as i32 * 24))
        .flat_map(|lane| {
            (0..4).flat_map(move |outgoing| {
                (0..4).filter_map(move |incoming| {
                    let from = departure + outgoing * 12;
                    let to = arrival - incoming * 12;
                    (from <= to).then(|| lane_route(start, exit, from, lane, to, end))
                })
            })
        })
        .filter(|points| {
            !points.windows(2).any(|segment| {
                builder
                    .scene
                    .nodes
                    .iter()
                    .any(|node| enters(segment[0], segment[1], node))
            })
        })
        .min_by_key(|points| {
            let length = points
                .windows(2)
                .map(|segment| {
                    (segment[0].x - segment[1].x).abs() + (segment[0].y - segment[1].y).abs()
                })
                .sum::<i32>();
            let crossings = routed
                .iter()
                .filter(|(other, _)| {
                    !(edge.from == other.from && edge.branch == other.branch || edge.to == other.to)
                })
                .flat_map(|(_, other)| other.windows(2))
                .map(|other| {
                    points
                        .windows(2)
                        .filter(|segment| crosses(segment[0], segment[1], other[0], other[1]))
                        .map(|segment| {
                            if (segment[0].y == segment[1].y) == (other[0].y == other[1].y) {
                                1000
                            } else {
                                1
                            }
                        })
                        .sum::<usize>()
                })
                .sum::<usize>();
            length + crossings as i32 * 720 + points.len() as i32 * 16
        })
        .expect("the outer routing lane clears every node")
}

fn lane_route(
    start: Point,
    exit: Point,
    departure: i32,
    lane: i32,
    arrival: i32,
    end: Point,
) -> Vec<Point> {
    let mut points = Vec::new();
    for point in compact_points([
        start,
        exit,
        Point {
            x: exit.x,
            y: departure,
        },
        Point {
            x: lane,
            y: departure,
        },
        Point {
            x: lane,
            y: arrival,
        },
        Point {
            x: end.x,
            y: arrival,
        },
        end,
    ]) {
        while points.len() >= 2 {
            let a: Point = points[points.len() - 2];
            let b: Point = points[points.len() - 1];
            if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) {
                points.pop();
            } else {
                break;
            }
        }
        points.push(point);
    }
    points
}

fn enters(a: Point, b: Point, node: &super::Node) -> bool {
    let left = node.x - node.width / 2;
    let right = node.x + node.width / 2;
    let top = node.y - node.height / 2;
    let bottom = node.y + node.height / 2;
    if a.x == b.x {
        a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
    } else {
        a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
    }
}

fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    let horizontal = a.y == b.y;
    if horizontal == (c.y == d.y) {
        if horizontal {
            a.y == c.y && a.x.min(b.x).max(c.x.min(d.x)) < a.x.max(b.x).min(c.x.max(d.x))
        } else {
            a.x == c.x && a.y.min(b.y).max(c.y.min(d.y)) < a.y.max(b.y).min(c.y.max(d.y))
        }
    } else {
        let (h1, h2, v1, v2) = if horizontal {
            (a, b, c, d)
        } else {
            (c, d, a, b)
        };
        v1.x >= h1.x.min(h2.x)
            && v1.x <= h1.x.max(h2.x)
            && h1.y >= v1.y.min(v2.y)
            && h1.y <= v1.y.max(v2.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_actions_join_without_duplicate_nodes_or_invented_order() {
        let source = r#"
            fn both(first: bool, second: bool) {
                #[action("Read the first flag")]
                |first| -> first_value { first };
                #[action("Read the second flag")]
                |second| -> second_value { second };
                #[action("Use both values")]
                |first_value, second_value| -> () { () };
                #[end]
                || {};
            }
        "#;
        let function = syn::parse_str(source).expect("the fixture parses");
        let graph = kaalang_model::build(&function).expect("the flow is valid");
        let builder = Builder {
            graph: &graph,
            scene: Scene::default(),
            indexes: HashMap::new(),
            terminals: Vec::new(),
            vertical_gap: super::super::label::vertical_gap(&graph),
        };
        let scene = layout(builder, "both(first: bool, second: bool)");
        assert_eq!(scene.nodes.len(), 5);
        assert!(
            !scene
                .edges
                .iter()
                .any(|edge| edge.from == NodeId::Block(0) && edge.to == NodeId::Block(1))
        );
        assert!(
            scene
                .edges
                .iter()
                .any(|edge| edge.from == NodeId::Block(0) && edge.to == NodeId::Block(2))
        );
        assert!(
            scene
                .edges
                .iter()
                .any(|edge| edge.from == NodeId::Block(1) && edge.to == NodeId::Block(2))
        );
    }

    /// The one accepted flow the branch tree cannot express, and therefore the
    /// only fixture this layout draws. Its routes must stay downward,
    /// orthogonal and clear of the nodes they pass.
    #[test]
    fn the_guarded_fixture_draws_unique_nodes_and_clear_routes() {
        let source =
            include_str!("../../../kaalang/tests/wire/behavior/question_after_a_partial_merge.rs");
        let file = syn::parse_file(source).expect("the fixture parses");
        let function = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function)
                    if function.sig.ident == "question_after_a_partial_merge" =>
                {
                    Some(function)
                }
                _ => None,
            })
            .expect("the fixture declares its flow");
        let graph = kaalang_model::build(function).expect("the fixture is valid");
        assert!(
            is_graph(&graph.execution_plan),
            "the fixture must still reach this layout"
        );
        let scene = super::super::layout(&graph, "question_after_a_partial_merge");
        assert_eq!(scene.nodes.len(), nodes(&graph).len());
        for edge in &scene.edges {
            let from = scene
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .expect("the origin is drawn");
            let to = scene
                .nodes
                .iter()
                .find(|node| node.id == edge.to)
                .expect("the destination is drawn");
            assert!(from.y < to.y);
            for segment in edge.points.windows(2) {
                assert!(segment[0].x == segment[1].x || segment[0].y == segment[1].y);
                assert!(segment[0].y <= segment[1].y);
                assert!(
                    !scene
                        .nodes
                        .iter()
                        .filter(|node| node.id != edge.from && node.id != edge.to)
                        .any(|node| enters(segment[0], segment[1], node))
                );
            }
        }
    }

    #[test]
    fn merged_capture_follows_branch_local_question_and_choice_exits() {
        let function = syn::parse_quote! {
            fn choose(condition: bool, local: bool) -> u8 {
                #[question("Choose a value")]
                |condition| -> (yes, no) { condition };
                #[action("Build yes")]
                |yes| -> (value, yes_work) { (1u8, ()) };
                #[action("Build no")]
                |no| -> (value, no_work) { (2u8, ()) };
                #[action("Use the merged value")]
                |value| -> result { value };
                #[question("Finish local yes work")]
                |yes_work, &local| -> (_yes_a, _yes_b) { *local };
                #[choice("Finish local no work")]
                #[case("First local result")]
                #[case("Second local result")]
                |no_work, local| -> (_no_a, _no_b) {
                    match local { true => (), false => () }
                };
                #[end]
                |result| {};
            }
        };
        let graph = kaalang_model::build(&function).expect("local work finishes before the merge");
        let connections = connections(&graph);
        let rows = rows(&nodes(&graph), &connections);
        for branch in 0..2 {
            for from in [NodeId::Block(4), NodeId::Case { choice: 5, branch }] {
                assert!(connections.contains(&Connection {
                    from,
                    branch: Some(branch),
                    to: NodeId::Block(3),
                }));
                assert!(rows[&from] < rows[&NodeId::Block(3)]);
            }
        }
        for block in [1, 2] {
            assert!(
                !connections.contains(&Connection {
                    from: NodeId::Block(block),
                    branch: None,
                    to: NodeId::Block(3),
                }),
                "local work makes the producer-to-consumer edge redundant"
            );
        }
    }
}
