use std::collections::{HashMap, HashSet};

use contour_model::{BlockKind, Graph, Merge, Plan};

const MARGIN: i32 = 48;
const CELL_WIDTH: i32 = 360;
const ROW_GAP: i32 = 100;
const NODE_WIDTH: i32 = 280;
/// Horizontal inset of a choice's slanted sides, shared with the SVG shape.
pub(crate) const CHOICE_SKEW: i32 = 28;
/// Font size of a node label, matching the stylesheet.
const LABEL_FONT: i32 = 14;
/// Font size of an edge label, matching the stylesheet.
pub(crate) const EDGE_LABEL_FONT: i32 = 12;
/// Text budget inside a rectangular node.
const NODE_LABEL_WIDTH: i32 = NODE_WIDTH - 32;
/// A diamond or hexagon narrows toward its points, so its text budget is smaller.
const BRANCH_LABEL_WIDTH: i32 = NODE_WIDTH - 96;
/// Text budget for an edge label, which floats free of any node.
pub(crate) const EDGE_LABEL_WIDTH: i32 = 240;

pub(crate) struct Scene {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeId {
    Start,
    Block(usize),
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Start,
    Action,
    Question,
    Choice,
    Merge,
    End,
}

pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) kind: NodeKind,
    pub(crate) label: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) lines: Vec<String>,
}

pub(crate) struct Edge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    pub(crate) label: Option<String>,
    pub(crate) start_x: i32,
    pub(crate) start_y: i32,
    pub(crate) middle_y: i32,
    pub(crate) side_x: Option<i32>,
    pub(crate) end_x: i32,
    pub(crate) end_y: i32,
}

pub(crate) fn layout(graph: &Graph) -> Scene {
    let parameters = graph
        .flow
        .sources
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let mut builder = Builder {
        graph,
        scene: Scene {
            width: 0,
            height: 0,
            nodes: vec![node(
                NodeId::Start,
                NodeKind::Start,
                format!("{}({parameters})", graph.name),
            )],
            edges: Vec::new(),
        },
        seen: HashSet::from([NodeId::Start]),
    };
    builder.walk(&graph.plan, NodeId::Start, None);
    debug_assert_eq!(builder.scene.nodes.len(), graph.flow.blocks.len() + 2);
    position(&mut builder.scene);

    builder.scene
}

struct Builder<'a> {
    graph: &'a Graph,
    scene: Scene,
    seen: HashSet<NodeId>,
}

impl Builder<'_> {
    fn walk(&mut self, plan: &Plan, from: NodeId, incoming_label: Option<String>) {
        match plan {
            Plan::Action { index, next } => {
                let node = NodeId::Block(*index);
                self.connect(from, node, incoming_label);
                self.walk(next, node, None);
            }
            Plan::Question {
                index,
                branches,
                merge,
            } => {
                let node = NodeId::Block(*index);
                self.connect(from, node, incoming_label);
                let block = &self.graph.flow.blocks[*index];
                for (branch_index, branch) in branches.iter().enumerate() {
                    let answer = if branch_index == 0 { "yes" } else { "no" };
                    let label = format!("{} / {answer}", block.outputs[branch_index]);
                    self.walk(&branch.plan, node, Some(label));
                }
                self.walk_merge(merge.as_ref());
            }
            Plan::Choice {
                index,
                branches,
                merge,
            } => {
                let node = NodeId::Block(*index);
                self.connect(from, node, incoming_label);
                let block = &self.graph.flow.blocks[*index];
                for (branch_index, branch) in branches.iter().enumerate() {
                    let label = format!(
                        "{} / {}",
                        block.outputs[branch_index], block.case_descriptions[branch_index]
                    );
                    self.walk(&branch.plan, node, Some(label));
                }
                self.walk_merge(merge.as_ref());
            }
            Plan::Terminal { output } => {
                // A terminal always follows its action, which passes no label down.
                debug_assert!(incoming_label.is_none());
                self.connect(from, NodeId::End, Some(output.to_string()));
            }
            Plan::Arrival { input, merge } => {
                let label = incoming_label.unwrap_or_else(|| input.to_string());
                self.connect(from, NodeId::Block(*merge), Some(label));
            }
        }
    }

    fn walk_merge(&mut self, merge: Option<&Merge>) {
        let Some(merge) = merge else {
            return;
        };
        let node = NodeId::Block(merge.index);
        self.ensure_node(node);
        let output = self.graph.flow.blocks[merge.index].outputs[0].to_string();
        self.walk(&merge.next, node, Some(output));
    }

    fn connect(&mut self, from: NodeId, to: NodeId, label: Option<String>) {
        self.ensure_node(to);
        self.scene.edges.push(Edge {
            from,
            to,
            label,
            start_x: 0,
            start_y: 0,
            middle_y: 0,
            side_x: None,
            end_x: 0,
            end_y: 0,
        });
    }

    fn ensure_node(&mut self, id: NodeId) {
        if !self.seen.insert(id) {
            return;
        }
        let layout_node = match id {
            NodeId::Start => unreachable!("the start node is created first"),
            NodeId::End => node(id, NodeKind::End, String::new()),
            NodeId::Block(index) => {
                let block = &self.graph.flow.blocks[index];
                node(
                    id,
                    match block.kind {
                        BlockKind::Action => NodeKind::Action,
                        BlockKind::Question => NodeKind::Question,
                        BlockKind::Choice => NodeKind::Choice,
                        BlockKind::Merge => NodeKind::Merge,
                    },
                    // A merge has no authored description and no drawn label.
                    block.description.clone().unwrap_or_default(),
                )
            }
        };
        self.scene.nodes.push(layout_node);
    }
}

fn node(id: NodeId, kind: NodeKind, label: String) -> Node {
    let (width, height, lines) = node_dimensions(kind, &label);
    Node {
        id,
        kind,
        label,
        x: 0,
        y: 0,
        width,
        height,
        lines,
    }
}

fn position(scene: &mut Scene) {
    let indexes = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect::<HashMap<_, _>>();
    let ranks = ranks(scene, &indexes);
    let rank_count = ranks.iter().copied().max().unwrap_or(0) + 1;
    let mut rows = vec![Vec::new(); rank_count];
    for (index, rank) in ranks.iter().copied().enumerate() {
        rows[rank].push(index);
    }

    let widest_row = rows.iter().map(Vec::len).max().unwrap_or(1) as i32;
    let width = MARGIN * 2 + widest_row * CELL_WIDTH;
    let row_heights = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|index| scene.nodes[*index].height)
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let mut row_centers = Vec::with_capacity(rows.len());
    let mut cursor = MARGIN;
    for height in &row_heights {
        row_centers.push(cursor + height / 2);
        cursor += height + ROW_GAP;
    }
    let height = cursor - ROW_GAP + MARGIN;

    for (rank, row) in rows.iter().enumerate() {
        let span = row.len() as i32 * CELL_WIDTH;
        let left = (width - span) / 2;
        for (column, index) in row.iter().copied().enumerate() {
            scene.nodes[index].x = left + column as i32 * CELL_WIDTH + CELL_WIDTH / 2;
            scene.nodes[index].y = row_centers[rank];
        }
    }

    let outgoing = edge_ordinals(&scene.edges, |edge| edge.from);
    let incoming = edge_ordinals(&scene.edges, |edge| edge.to);
    for (edge_index, edge) in scene.edges.iter_mut().enumerate() {
        let from_index = indexes[&edge.from];
        let to_index = indexes[&edge.to];
        let from = &scene.nodes[from_index];
        let to = &scene.nodes[to_index];
        let (out_index, out_count) = outgoing[edge_index];
        let (in_index, in_count) = incoming[edge_index];
        (edge.start_x, edge.start_y) = vertical_anchor(from, out_index, out_count, 1);
        (edge.end_x, edge.end_y) = vertical_anchor(to, in_index, in_count, -1);
        edge.middle_y = edge.start_y + (edge.end_y - edge.start_y) / 2;
        edge.side_x = (ranks[to_index] > ranks[from_index] + 1).then(|| {
            if from.x < width / 2 {
                MARGIN
            } else {
                width - MARGIN
            }
        });
    }

    scene.width = width;
    scene.height = height;
}

fn ranks(scene: &Scene, indexes: &HashMap<NodeId, usize>) -> Vec<usize> {
    let mut ranks = vec![0; scene.nodes.len()];
    let mut incoming = vec![0; scene.nodes.len()];
    for edge in &scene.edges {
        incoming[indexes[&edge.to]] += 1;
    }
    let mut ready = vec![indexes[&NodeId::Start]];
    let mut cursor = 0;
    while cursor < ready.len() {
        let from = ready[cursor];
        cursor += 1;
        for edge in scene
            .edges
            .iter()
            .filter(|edge| indexes[&edge.from] == from)
        {
            let to = indexes[&edge.to];
            ranks[to] = ranks[to].max(ranks[from] + 1);
            incoming[to] -= 1;
            if incoming[to] == 0 {
                ready.push(to);
            }
        }
    }
    debug_assert_eq!(ready.len(), scene.nodes.len());

    ranks
}

fn node_dimensions(kind: NodeKind, label: &str) -> (i32, i32, Vec<String>) {
    match kind {
        NodeKind::Merge => (38, 38, vec!["M".to_owned()]),
        NodeKind::End => (180, 58, Vec::new()),
        NodeKind::Start => {
            let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);
            let height = 58.max(42 + lines.len() as i32 * 18);
            (NODE_WIDTH, height, lines)
        }
        NodeKind::Action => block_dimensions(label, NODE_LABEL_WIDTH, 78),
        NodeKind::Question | NodeKind::Choice => block_dimensions(label, BRANCH_LABEL_WIDTH, 112),
    }
}

fn block_dimensions(label: &str, budget: i32, minimum_height: i32) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, budget, LABEL_FONT);
    let height = minimum_height.max(48 + lines.len() as i32 * 18);
    (NODE_WIDTH, height, lines)
}

/// Approximate advance width of one character, in hundredths of an em, for the
/// sans-serif stack the stylesheet requests. The renderer has no font metrics,
/// so this estimate errs wide rather than letting a label leave its node.
fn advance(character: char) -> i32 {
    match character {
        '\t' => 200,
        ' ' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '`' | 'i' | 'j' | 'l' | 'I' | '('
        | ')' | '[' | ']' | '{' | '}' | '/' | '\\' | '-' => 32,
        'm' | 'w' | 'M' | 'W' | '@' => 90,
        'A'..='Z' => 68,
        _ if character.is_ascii() => 56,
        // Latin-1, Greek, and Cyrillic behave like Latin; assume anything
        // beyond them, such as CJK, is full width.
        _ if (character as u32) < 0x0500 => 60,
        _ => 100,
    }
}

/// Estimated rendered width of `characters` at `font_size`.
fn text_width(characters: &[char], font_size: i32) -> i32 {
    characters.iter().copied().map(advance).sum::<i32>() * font_size / 100
}

/// Returns how many leading characters fit within `budget`, at least one so
/// that wrapping always makes progress.
fn fitting_count(characters: &[char], budget: i32, font_size: i32) -> usize {
    let mut used = 0;
    for (count, character) in characters.iter().enumerate() {
        used += advance(*character);
        if used * font_size / 100 > budget {
            return count.max(1);
        }
    }

    characters.len()
}

/// Breaks a label into lines that fit `budget` pixels at `font_size`, without
/// changing the authored text.
pub(crate) fn wrap_text(text: &str, budget: i32, font_size: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let characters = paragraph.chars().collect::<Vec<_>>();
        if characters.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut start = 0;
        while text_width(&characters[start..], font_size) > budget {
            let hard_end = start + fitting_count(&characters[start..], budget, font_size);
            let preferred_break = characters[start..hard_end]
                .iter()
                .rposition(|character| character.is_whitespace())
                .map(|offset| start + offset + 1)
                .filter(|end| *end > start + 1);
            let next_break = characters[hard_end..]
                .iter()
                .position(|character| character.is_whitespace())
                .map(|offset| hard_end + offset + 1);
            let ascii_break = characters[start..hard_end]
                .iter()
                .all(char::is_ascii)
                .then_some(hard_end);
            let Some(end) = preferred_break.or(next_break).or(ascii_break) else {
                // ponytail: unspaced non-ASCII stays intact until layout gains
                // real grapheme segmentation and font metrics.
                break;
            };
            lines.push(characters[start..end].iter().collect());
            start = end;
        }
        if start < characters.len() {
            lines.push(characters[start..].iter().collect());
        }
    }

    lines
}

fn edge_ordinals(edges: &[Edge], endpoint: impl Fn(&Edge) -> NodeId) -> Vec<(usize, usize)> {
    let mut totals = HashMap::new();
    for edge in edges {
        *totals.entry(endpoint(edge)).or_insert(0) += 1;
    }
    let mut seen = HashMap::new();
    edges
        .iter()
        .map(|edge| {
            let endpoint = endpoint(edge);
            let index = seen.entry(endpoint).or_insert(0);
            let ordinal = *index;
            *index += 1;
            (ordinal, totals[&endpoint])
        })
        .collect()
}

fn anchor_x(center: i32, width: i32, ordinal: usize, count: usize) -> i32 {
    if count == 1 || width < 80 {
        center
    } else {
        center - width / 2 + width * (ordinal as i32 + 1) / (count as i32 + 1)
    }
}

fn vertical_anchor(node: &Node, ordinal: usize, count: usize, direction: i32) -> (i32, i32) {
    let anchor_width = match node.kind {
        NodeKind::Question => node.width,
        NodeKind::Choice => node.width - 2 * CHOICE_SKEW,
        NodeKind::Start | NodeKind::End => node.width - node.height,
        NodeKind::Action => node.width - 16,
        NodeKind::Merge => 0,
    };
    let x = anchor_x(node.x, anchor_width, ordinal, count);
    let half_height = node.height / 2;
    let y_offset = if node.kind == NodeKind::Question {
        let half_width = node.width / 2;
        half_height * (half_width - (x - node.x).abs()) / half_width
    } else {
        half_height
    };

    (x, node.y + direction * y_offset)
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    use super::{LABEL_FONT, NODE_LABEL_WIDTH, NodeId, NodeKind, layout, text_width, wrap_text};

    /// The control-flow connections a diagram draws, in walk order.
    fn connections(source: &str, flow: &str) -> Vec<(NodeId, NodeId)> {
        let file = syn::parse_file(source).expect("the fixture parses");
        let syn::Item::Fn(function) = file
            .items
            .iter()
            .find(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == flow))
            .expect("the fixture declares the flow")
        else {
            unreachable!("the item was matched as a function")
        };
        let graph = contour_model::build(function).expect("the flow is valid");

        layout(&graph)
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect()
    }

    #[test]
    fn wrapping_does_not_split_unspaced_unicode_text() {
        assert_eq!(wrap_text("драконоподобный", 20, 14), ["драконоподобный"]);
        assert_eq!(wrap_text("👩‍💻👩‍💻", 8, 14), ["👩‍💻👩‍💻"]);
    }

    #[test]
    fn wrapping_preserves_authored_whitespace() {
        let text = "  exact  spacing  ";

        assert_eq!(wrap_text(text, 40, 14).concat(), text);
    }

    #[test]
    fn wrapped_lines_fit_the_label_budget() {
        // Wide uppercase overflowed a fixed character limit; every line must now
        // fit the node, and the authored text must survive the wrap unchanged.
        let label = "REJECT THE WWWWIDE APPLICATION IMMEDIATELY AND WITHOUT DELAY";
        let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        for line in &lines {
            let characters = line.chars().collect::<Vec<_>>();
            assert!(
                text_width(&characters, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {line}"
            );
        }
        assert_eq!(lines.concat(), label);
    }

    #[test]
    fn wrapping_splits_an_unspaced_wide_ascii_label() {
        let label = "W".repeat(32);
        let lines = wrap_text(&label, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| {
            text_width(&line.chars().collect::<Vec<_>>(), LABEL_FONT) <= NODE_LABEL_WIDTH
        }));
        assert_eq!(lines.concat(), label);
    }

    #[test]
    fn nested_branches_converge_at_their_own_merge() {
        use NodeId::{Block, End, Start};

        let source = r#"
            #[contour]
            fn nested(outer: bool, inner: bool) -> u8 {
                #[question("Take the outer path?")]
                |outer| -> (outer_yes, outer_no) { outer };

                #[question("Take the inner path?")]
                |outer_yes, inner| -> (inner_yes, inner_no) { inner };

                #[action("Build the inner yes value")]
                |inner_yes| -> inner_yes_value { 1 };

                #[action("Build the inner no value")]
                |inner_no| -> inner_no_value { 2 };

                #[merge]
                |inner_yes_value, inner_no_value| -> inner_value {};

                #[action("Finish the inner path")]
                |inner_value| -> outer_yes_value { inner_value };

                #[action("Build the outer no value")]
                |outer_no| -> outer_no_value { 0 };

                #[merge]
                |outer_yes_value, outer_no_value| -> outer_value {};

                #[action("Return the result")]
                |outer_value| -> result { outer_value };
            }
        "#;

        assert_eq!(
            connections(source, "nested"),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(1), Block(2)),
                (Block(2), Block(4)),
                (Block(1), Block(3)),
                (Block(3), Block(4)),
                (Block(4), Block(5)),
                (Block(5), Block(7)),
                (Block(0), Block(6)),
                (Block(6), Block(7)),
                (Block(7), Block(8)),
                (Block(8), End),
            ]
        );
    }

    #[test]
    fn a_terminal_sibling_reaches_the_end_beside_a_merge() {
        use NodeId::{Block, End, Start};

        let source = r#"
            #[contour]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Right")]
                #[case("Finish now")]
                |input| -> (left, right, done) {
                    match input {
                        0 => (),
                        1 => (),
                        _ => (),
                    }
                };

                #[action("Build left")]
                |left| -> left_value { 1 };

                #[action("Build right")]
                |right| -> right_value { 2 };

                #[action("Finish immediately")]
                |done| -> immediate_result { 3 };

                #[merge]
                |left_value, right_value| -> selected {};

                #[action("Finish after merge")]
                |selected| -> merged_result { selected };
            }
        "#;

        assert_eq!(
            connections(source, "partial"),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(1), Block(4)),
                (Block(0), Block(2)),
                (Block(2), Block(4)),
                (Block(0), Block(3)),
                (Block(3), End),
                (Block(4), Block(5)),
                (Block(5), End),
            ]
        );
    }

    #[test]
    fn question_edges_start_on_the_diamond_perimeter() {
        let function: ItemFn = parse_quote! {
            fn decide(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (yes, no) { condition };

                #[action("Return yes")]
                |yes| -> yes_result { 1 };

                #[action("Return no")]
                |no| -> no_result { 0 };
            }
        };
        let graph = contour_model::build(&function).unwrap();
        let scene = layout(&graph);
        let question = scene
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Question)
            .unwrap();
        let edges = scene
            .edges
            .iter()
            .filter(|edge| edge.from == NodeId::Block(0))
            .collect::<Vec<_>>();

        assert_eq!(edges.len(), 2);
        for edge in edges {
            let horizontal = (edge.start_x - question.x).abs();
            let expected_vertical =
                (question.height / 2) * (question.width / 2 - horizontal) / (question.width / 2);
            assert_eq!(edge.start_y - question.y, expected_vertical);
        }
    }
}
