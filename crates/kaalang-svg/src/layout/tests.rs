//! Checks the placed scene of whole flows: which nodes and connections a
//! plan draws, and where its labels land.

use syn::{ItemFn, parse_quote};

use super::{
    EDGE_LABEL_FONT, EDGE_LABEL_HALO, EDGE_LINE_HEIGHT, Node, NodeId, NodeKind, Point, Scene,
    label::{label_bounds, label_width},
    layout, signature_text, skewer_x,
};

fn scene(source: &str, flow: &str) -> Scene {
    let file = syn::parse_file(source).expect("the fixture parses");
    let syn::Item::Fn(function) = file
        .items
        .iter()
        .find(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == flow))
        .expect("the fixture declares the flow")
    else {
        unreachable!("the item was matched as a function")
    };
    let graph = kaalang_model::build(function).expect("the flow is valid");

    layout(&graph, &signature_text(source, &function.sig))
}

fn node(scene: &Scene, id: NodeId) -> &Node {
    scene
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("the node is drawn")
}

/// Start carries the signature as authored. `fn` goes because the label is
/// already known to be the flow, a signature spread over source lines
/// becomes one line so the label wraps to its own budget, and everything
/// else survives: a wildcard parameter the boundary declares no wire for,
/// a raw identifier, generics, and a where clause.
#[test]
fn a_signature_label_keeps_the_authored_text_without_the_fn_keyword() {
    let source = r"
        #[kaalang]
        fn boundary<T>(
            _: u8,
            r#type: T,
        ) -> T
        where
            T: Clone,
        {
            #[end]
            |r#type| {};
        }
    ";
    let file = syn::parse_file(source).expect("the fixture parses");
    let syn::Item::Fn(function) = &file.items[0] else {
        unreachable!("the fixture declares a function")
    };

    assert_eq!(
        signature_text(source, &function.sig),
        "boundary<T>( _: u8, r#type: T, ) -> T where T: Clone,"
    );
}

/// Start reaches End only across a wire End captures. What the boundary
/// declares does not decide it: a wildcard parameter, a named parameter
/// left unconsumed, and no parameter at all are drawn the same way.
#[test]
fn a_zero_computation_flow_connects_start_to_end_only_through_a_captured_wire() {
    for parameters in ["", "_: u8", "_value: u8"] {
        let source = format!(
            r"
            #[kaalang]
            fn boundary({parameters}) {{
                #[end]
                || {{}};
            }}
        "
        );
        let scene = scene(&source, "boundary");

        assert_eq!(
            scene.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            [NodeId::Start, NodeId::Block(0)]
        );
        assert_eq!(node(&scene, NodeId::Block(0)).kind, NodeKind::End);
        assert!(
            scene.edges.is_empty(),
            "`{parameters}` hands nothing to End, so nothing connects them"
        );
    }

    let scene = scene(
        r"
            #[kaalang]
            fn identity<T>(value: T) -> T {
                #[end]
                |value| {};
            }
        ",
        "identity",
    );

    assert_eq!(node(&scene, NodeId::Block(0)).kind, NodeKind::End);
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (
                edge.from,
                edge.to,
                edge.handover.clone(),
                edge.capture.clone()
            ))
            .collect::<Vec<_>>(),
        [(
            NodeId::Start,
            NodeId::Block(0),
            vec![String::from("value")],
            vec![String::from("value")],
        )]
    );
}

#[test]
fn a_consumed_underscore_wire_keeps_its_name() {
    let scene = scene(
        r"
            #[kaalang]
            fn identity(_value: u8) -> u8 {
                #[end]
                |_value| {};
            }
        ",
        "identity",
    );

    assert_eq!(scene.edges[0].handover, ["_value"]);
    assert_eq!(scene.edges[0].capture, ["_value"]);
    assert_eq!(scene.labels[0].lines.concat(), "_value");
}

/// A choice hands over one output per connection, so the gap it needs
/// follows its widest single wire. Lengthening the other two must not
/// reserve room for a label that names all three at once.
#[test]
fn a_choice_reserves_no_room_for_its_outputs_joined() {
    fn lanes(wires: [&str; 3]) -> String {
        format!(
            r#"
            #[kaalang]
            fn lanes(request: u8) {{
                #[choice("Pick a lane.")]
                #[case("A")]
                #[case("B")]
                #[case("C")]
                |&request| -> ({0}, {1}, {2}) {{
                    match request {{
                        1 => (),
                        2 => (),
                        _ => (),
                    }}
                }};

                #[action("Alpha.")]
                |{0}| -> result {{}};

                #[action("Bravo.")]
                |{1}| -> result {{}};

                #[action("Charlie.")]
                |{2}| -> result {{}};

                #[end]
                |result| {{}};
            }}
            "#,
            wires[0], wires[1], wires[2]
        )
    }

    // The first wire is the widest in both flows, so both need the same
    // gap. Only the joined width differs, and only one of them wraps.
    let widest = "a_lane_wire_name_long_enough_to_wrap_on_its_own";
    let one_long = scene(&lanes([widest, "b", "c"]), "lanes");
    let all_long = scene(
        &lanes([widest, "b_lane_wire_shorter", "c_lane_wire_shorter"]),
        "lanes",
    );

    assert_eq!(all_long.height, one_long.height);
}

/// A flow whose endpoint labels need more than the minimum vertical gap,
/// including a wrapped hand-over leaving a question horizontally.
fn wrapping_labels() -> Scene {
    let source = r#"
        #[kaalang]
        fn wide(
            condition: bool,
            a_second_boundary_wire_that_makes_the_connection_label_wrap: u8,
            a_third_boundary_wire_that_makes_the_connection_label_wrap: u8,
            a_fourth_boundary_wire_that_makes_the_connection_label_wrap: u8,
            a_fifth_boundary_wire_that_makes_the_connection_label_wrap: u8,
        ) -> u8 {
            #[question("Choose a path")]
            |condition| -> (accepted, a_rejected_branch_wire_name_long_enough_to_wrap_beside_its_horizontal_exit) { condition };

            #[action("Take the accepted path")]
            |accepted,
             a_second_boundary_wire_that_makes_the_connection_label_wrap,
             a_third_boundary_wire_that_makes_the_connection_label_wrap,
             a_fourth_boundary_wire_that_makes_the_connection_label_wrap,
             a_fifth_boundary_wire_that_makes_the_connection_label_wrap|
                -> a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines { 1 };

            #[action("Take the rejected path")]
            |a_rejected_branch_wire_name_long_enough_to_wrap_beside_its_horizontal_exit,
             a_second_boundary_wire_that_makes_the_connection_label_wrap,
             a_third_boundary_wire_that_makes_the_connection_label_wrap,
             a_fourth_boundary_wire_that_makes_the_connection_label_wrap,
             a_fifth_boundary_wire_that_makes_the_connection_label_wrap|
                -> a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines {
                0
            };

            #[end]
            |a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines| {};
        }
    "#;

    let scene = scene(source, "wide");
    assert!(
        scene.labels.iter().any(|label| label.lines.len() >= 6),
        "the fixture must need more than the minimum connection gap"
    );
    assert!(
        scene.labels.iter().any(|label| {
            label.lines.concat().starts_with(
                "a_rejected_branch_wire_name_long_enough_to_wrap_beside_its_horizontal_exit",
            ) && label.lines.len() > 1
        }),
        "the horizontal hand-over must wrap"
    );

    scene
}

/// A branch wire long enough to wrap on the horizontal exit it leaves by,
/// and consumed under its own name, so one shared label sits beside the
/// branch point rather than at either end of the run.
fn wrapping_shared_label() -> Scene {
    let source = r#"
        #[kaalang]
        fn wide(condition: bool) -> u8 {
            #[question("Choose a path")]
            |condition| -> (accepted, a_rejected_branch_wire_name_long_enough_to_wrap_several_times_beside_its_horizontal_exit) { condition };

            #[action("Take the accepted path")]
            |accepted| -> result { 1 };

            #[action("Take the rejected path")]
            |a_rejected_branch_wire_name_long_enough_to_wrap_several_times_beside_its_horizontal_exit| -> result { 0 };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "wide");
    assert!(
        scene.labels.iter().any(|label| label.lines.len() > 1),
        "the fixture must wrap the shared label on the horizontal exit"
    );

    scene
}

/// A connection label stays drawable however much vertical room it needs.
#[test]
fn a_wrapped_connection_label_stays_inside_the_canvas() {
    for scene in [wrapping_labels(), wrapping_shared_label()] {
        for label in &scene.labels {
            let width = label_width(&label.lines);
            let left = label.at.x - width / 2 - EDGE_LABEL_HALO;
            let top = label.at.y - EDGE_LABEL_FONT / 2 - EDGE_LABEL_HALO;
            let (right, bottom) = label_bounds(label);

            assert!(left >= 0, "a label leaves the canvas: {left}");
            assert!(top >= 0, "a label leaves the canvas: {top}");
            assert!(right <= scene.width, "a label leaves the canvas: {right}");
            assert!(
                bottom <= scene.height,
                "a label leaves the canvas: {bottom}"
            );
        }
    }
}

/// Nodes are drawn after the labels and fill themselves white, so a line
/// that reaches into one is painted over. A label stacks away from the node
/// it names to keep every wrapped line outside it. Only the glyphs are
/// measured: the halo is white on white, so a node covering its edge costs
/// nothing.
#[test]
fn a_wrapped_connection_label_stays_outside_every_node() {
    for scene in [wrapping_labels(), wrapping_shared_label()] {
        for label in &scene.labels {
            let width = label_width(&label.lines);
            let last_baseline = label.at.y + (label.lines.len() as i32 - 1) * EDGE_LINE_HEIGHT;
            let left = label.at.x - width / 2;
            let right = label.at.x + width / 2;
            let top = label.at.y - EDGE_LABEL_FONT / 2;
            let bottom = last_baseline + EDGE_LABEL_FONT / 2;

            for node in &scene.nodes {
                let overlaps = left < node.x + node.width / 2
                    && right > node.x - node.width / 2
                    && top < node.y + node.height / 2
                    && bottom > node.y - node.height / 2;
                assert!(
                    !overlaps,
                    "the label {:?} at {left}..{right} x {top}..{bottom} reaches into the {:?} node at {},{}",
                    label.lines, node.kind, node.x, node.y
                );
            }
        }
    }
}

#[test]
fn nested_branches_converge_at_their_own_consumers() {
    use NodeId::{Block, Start};

    let source = r#"
        #[kaalang]
        fn nested(outer: bool, inner: bool) -> u8 {
            #[question("Take the outer path?")]
            |outer, &inner| -> (outer_yes, outer_no) { outer };

            #[question("Take the inner path?")]
            |outer_yes, inner| -> (inner_yes, inner_no) { inner };

            #[action("Build the inner yes value")]
            |inner_yes| -> inner_value { 1 };

            #[action("Build the inner no value")]
            |inner_no| -> inner_value { 2 };

            #[action("Produce the inner-path value")]
            |inner_value| -> outer_value { inner_value };

            #[action("Build the outer no value")]
            |outer_no| -> outer_value { 0 };

            #[action("Produce the result")]
            |outer_value| -> result { outer_value };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "nested");
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect::<Vec<_>>(),
        [
            (Start, Block(0)),
            (Block(0), Block(1)),
            (Block(1), Block(2)),
            (Block(1), Block(3)),
            (Block(2), Block(4)),
            (Block(3), Block(4)),
            (Block(0), Block(5)),
            (Block(4), Block(6)),
            (Block(5), Block(6)),
            (Block(6), Block(7)),
        ]
    );
    assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(1)).x);
    assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(4)).x);
    assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(6)).x);
    assert!(node(&scene, Block(3)).x < node(&scene, Block(5)).x);
}

#[test]
fn implicit_convergence_places_the_shared_consumer_once() {
    use NodeId::{Block, Start};

    let source = r#"
        #[kaalang]
        fn choose(condition: bool) -> u8 {
            #[question("Choose a path")]
            |condition| -> (yes, no) { condition };

            #[action("Build yes")]
            |yes| -> selected { 1 };

            #[action("Build no")]
            |no| -> selected { 2 };

            #[action("Use selected")]
            |selected| -> result { selected };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "choose");
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect::<Vec<_>>(),
        [
            (Start, Block(0)),
            (Block(0), Block(1)),
            (Block(0), Block(2)),
            (Block(1), Block(3)),
            (Block(2), Block(3)),
            (Block(3), Block(4)),
        ]
    );
    assert_eq!(
        scene
            .nodes
            .iter()
            .filter(|node| node.id == Block(3))
            .count(),
        1
    );
    assert_eq!(
        scene
            .edges
            .iter()
            .filter(|edge| edge.to == Block(3))
            .map(|edge| edge.capture.join(", "))
            .collect::<Vec<_>>(),
        ["selected", "selected"]
    );
}

#[test]
fn nested_convergence_routes_every_branch_into_one_consumer() {
    use NodeId::{Block, Start};

    let source = r#"
        #[kaalang]
        fn choose(outer: bool, inner: bool) -> u8 {
            #[question("Take the nested path?")]
            |outer, &inner| -> (nested, direct) { outer };

            #[question("Choose the nested value")]
            |nested, inner| -> (inner_yes, inner_no) { inner };

            #[action("Build nested yes")]
            |inner_yes| -> selected { 1 };

            #[action("Build nested no")]
            |inner_no| -> selected { 2 };

            #[action("Build direct")]
            |direct| -> selected { 3 };

            #[action("Use selected")]
            |selected| -> result { selected };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "choose");
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect::<Vec<_>>(),
        [
            (Start, Block(0)),
            (Block(0), Block(1)),
            (Block(1), Block(2)),
            (Block(1), Block(3)),
            (Block(0), Block(4)),
            (Block(2), Block(5)),
            (Block(3), Block(5)),
            (Block(4), Block(5)),
            (Block(5), Block(6)),
        ]
    );
    assert_eq!(
        scene
            .nodes
            .iter()
            .filter(|node| node.id == Block(5))
            .count(),
        1
    );
    // The shared consumer takes the first continuing branch's skewer.
    assert_eq!(node(&scene, Block(2)).x, node(&scene, Block(5)).x);
    for edge in &scene.edges {
        for segment in edge.points.windows(2) {
            for node in &scene.nodes {
                assert!(
                    !enters_node(segment, node),
                    "{:?} crosses {:?}",
                    edge.from,
                    node.id
                );
            }
        }
    }
}

#[test]
fn a_nested_branch_point_without_its_own_join_hands_its_tails_outward() {
    use NodeId::{Block, Start};

    let source = r#"
        #[kaalang]
        fn choose(outer: bool, inner: bool) -> u8 {
            #[question("Take the nested path?")]
            |outer, &inner| -> (nested, direct) { outer };

            #[question("Choose the nested depth")]
            |nested, inner| -> (short, long) { inner };

            #[action("Build the short value")]
            |short| -> selected { 1 };

            #[action("Prepare the long value")]
            |long| -> prepared { 2 };

            #[action("Build the direct value")]
            |direct| -> selected { 3 };

            #[action("Build the long value")]
            |prepared| -> selected { 4 };

            #[action("Use selected")]
            |selected| -> result { selected };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "choose");
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect::<Vec<_>>(),
        [
            (Start, Block(0)),
            (Block(0), Block(1)),
            (Block(1), Block(2)),
            (Block(1), Block(3)),
            (Block(3), Block(5)),
            (Block(0), Block(4)),
            (Block(2), Block(6)),
            (Block(5), Block(6)),
            (Block(4), Block(6)),
            (Block(6), Block(7)),
        ]
    );
}

#[test]
fn a_nested_branch_bypasses_its_local_join_for_the_outer_continuation() {
    let scene = scene(
        include_str!(
            "../../../kaalang/tests/wire/behavior/nested_branch_passes_a_question_join.rs"
        ),
        "nested_branch_passes_a_question_join",
    );
    let destinations = |from| {
        scene
            .edges
            .iter()
            .filter(|edge| edge.from == NodeId::Block(from))
            .map(|edge| edge.to)
            .collect::<Vec<_>>()
    };

    assert_eq!(destinations(6), [NodeId::Block(8)]);
    assert_eq!(destinations(5), [NodeId::Block(9)]);
    assert_eq!(destinations(3), [NodeId::Block(5)]);
    assert_eq!(destinations(4), [NodeId::Block(5)]);
}

#[test]
fn choice_uses_ordered_case_nodes_and_output_only_labels() {
    use NodeId::{Block, Case};

    let source = r#"
        #[kaalang]
        fn choose(input: u8) -> u8 {
            #[choice("Choose a path")]
            #[case("Left")]
            #[case("Middle")]
            #[case("Right")]
            |input| -> (left, middle, right) {
                match input { 0 => (), 1 => (), _ => () }
            };

            #[action("Build left")]
            |left| -> result { 0 };

            #[action("Build middle")]
            |middle| -> result { 1 };

            #[action("Build right")]
            |right| -> result { 2 };

            #[end]
            |result| {};
        }
    "#;
    let scene = scene(source, "choose");
    let select_edges = scene
        .edges
        .iter()
        .filter(|edge| edge.from == Block(0))
        .collect::<Vec<_>>();

    assert_eq!(select_edges.len(), 3);
    assert!(
        select_edges
            .iter()
            .all(|edge| edge.handover.is_empty() && edge.capture.is_empty())
    );
    assert_eq!(
        select_edges.iter().map(|edge| edge.to).collect::<Vec<_>>(),
        [
            Case {
                choice: 0,
                branch: 0
            },
            Case {
                choice: 0,
                branch: 1
            },
            Case {
                choice: 0,
                branch: 2
            },
        ]
    );
    let case_edges = scene
        .edges
        .iter()
        .filter(|edge| matches!(edge.from, Case { .. }))
        .collect::<Vec<_>>();
    assert_eq!(
        case_edges
            .iter()
            .map(|edge| edge.handover.join(", "))
            .collect::<Vec<_>>(),
        ["left", "middle", "right"]
    );
    let case_x = scene
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Case)
        .map(|node| node.x)
        .collect::<Vec<_>>();
    assert!(case_x.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_terminal_sibling_reaches_the_end_beside_a_convergence() {
    use NodeId::{Block, Case, Start};

    let source = r#"
        #[kaalang]
        fn partial(input: u8) -> u8 {
            #[choice("Choose a path")]
            #[case("Left")]
            #[case("Right")]
            #[case("Reach End directly")]
            |input| -> (left, right, done) {
                match input { 0 => (), 1 => (), _ => () }
            };

            #[action("Build left")]
            |left| -> selected { 1 };

            #[action("Build right")]
            |right| -> selected { 2 };

            #[action("Produce the direct result")]
            |done| -> result { 3 };

            #[action("Produce the converged result")]
            |selected| -> result { selected };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "partial");
    assert_eq!(
        scene
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect::<Vec<_>>(),
        [
            (Start, Block(0)),
            (
                Block(0),
                Case {
                    choice: 0,
                    branch: 0
                }
            ),
            (
                Block(0),
                Case {
                    choice: 0,
                    branch: 1
                }
            ),
            (
                Block(0),
                Case {
                    choice: 0,
                    branch: 2
                }
            ),
            (
                Case {
                    choice: 0,
                    branch: 0
                },
                Block(1)
            ),
            (
                Case {
                    choice: 0,
                    branch: 1
                },
                Block(2)
            ),
            (
                Case {
                    choice: 0,
                    branch: 2
                },
                Block(3)
            ),
            (Block(1), Block(4)),
            (Block(2), Block(4)),
            (Block(3), Block(5)),
            (Block(4), Block(5)),
        ]
    );
    let early_terminal = scene
        .edges
        .iter()
        .find(|edge| edge.from == Block(3) && edge.to == Block(5))
        .expect("the early terminal reaches End");
    assert_eq!(early_terminal.points.len(), 4);
    assert_eq!(early_terminal.points[0].x, early_terminal.points[1].x);
    assert_eq!(early_terminal.points[1].x, node(&scene, Block(3)).x);
    assert_eq!(early_terminal.points[2].x, node(&scene, Block(5)).x);
}

/// Terminal branches meet in one collector above End: each
/// drops onto its shared row, and the collector makes the single descent
/// into the node.
#[test]
fn terminal_branches_share_one_collector_into_end() {
    let source = r#"
        #[kaalang]
        fn partial(input: u8) -> u8 {
            #[choice("Choose a path")]
            #[case("First End path")]
            #[case("Second End path")]
            #[case("Third End path")]
            #[case("Fourth End path")]
            |input| -> (first, second, third, fourth) {
                match input { 0 => (), 1 => (), 2 => (), _ => () }
            };

            #[action("Build first")]
            |first| -> result { 1 };

            #[action("Build second")]
            |second| -> result { 2 };

            #[action("Build third")]
            |third| -> result { 3 };

            #[action("Build fourth")]
            |fourth| -> result { 4 };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "partial");
    let end_top = node(&scene, NodeId::Block(5));
    let end_top = Point {
        x: end_top.x,
        y: end_top.y - end_top.height / 2,
    };
    let terminals = scene
        .edges
        .iter()
        .filter(|edge| edge.to == NodeId::Block(5))
        .collect::<Vec<_>>();

    assert_eq!(terminals.len(), 4);

    // Every branch away from the column turns onto the collector, and the
    // collector makes one descent that the branch already in the column
    // drops straight through.
    let turning = terminals
        .iter()
        .filter(|edge| edge.points.len() > 2)
        .collect::<Vec<_>>();
    let straight = terminals
        .iter()
        .find(|edge| edge.points.len() == 2)
        .expect("the branch above End drops straight in");

    assert_eq!(turning.len(), 3);
    let collector_y = turning[0].points[turning[0].points.len() - 2].y;
    let descent = [
        Point {
            x: end_top.x,
            y: collector_y,
        },
        end_top,
    ];
    for edge in &turning {
        let corner = edge.points[edge.points.len() - 2];
        assert_eq!(
            corner.y, collector_y,
            "{:?} leaves the collector",
            edge.from
        );
        assert_eq!(
            [corner, edge.points[edge.points.len() - 1]],
            descent,
            "{:?} makes its own descent",
            edge.from
        );
    }
    assert_eq!(straight.points[1], end_top);
    assert_eq!(straight.points[0].x, end_top.x);
    assert!(straight.points[0].y <= collector_y);

    for edge in terminals {
        for segment in edge.points.windows(2) {
            for node in &scene.nodes {
                assert!(
                    !enters_node(segment, node),
                    "{:?} crosses {:?}: {:?}",
                    edge.from,
                    node.id,
                    edge.points
                );
            }
        }
    }
}

/// A branch that ends the flow may lead the ones that converge, and it keeps
/// the branching block's own skewer. The shared continuation takes the
/// first continuing branch's skewer.
#[test]
fn a_leading_end_path_keeps_the_shared_continuation_off_its_skewer() {
    let source = r#"
        #[kaalang]
        fn partial(input: u8) -> u8 {
            #[choice("Choose a path")]
            #[case("Reach End directly")]
            #[case("Left")]
            #[case("Right")]
            |input| -> (done, left, right) {
                match input { 0 => (), 1 => (), _ => () }
            };

            #[action("Produce the direct result")]
            |done| -> result { 1 };

            #[action("Build left")]
            |left| -> selected { 2 };

            #[action("Build right")]
            |right| -> selected { 3 };

            #[action("Produce the converged result")]
            |selected| -> result { selected };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "partial");
    assert_eq!(node(&scene, NodeId::Block(1)).x, skewer_x(0));
    assert_eq!(node(&scene, NodeId::Block(4)).x, skewer_x(1));
}

/// A shared continuation reserves its full footprint before a trailing
/// terminal branch, even when the continuing siblings themselves are narrow.
#[test]
fn a_wide_continuation_moves_a_trailing_terminal_past_its_footprint() {
    use NodeId::Block;

    let source = r#"
        #[kaalang]
        fn partial(input: u8) -> u8 {
            #[choice("Choose a path")]
            #[case("Left")]
            #[case("Right")]
            #[case("Reach End directly")]
            |input| -> (left, right, done) {
                match input { 0 => (), 1 => (), _ => () }
            };

            #[action("Build left")]
            |left| -> selected { 1 };

            #[action("Build right")]
            |right| -> selected { 2 };

            #[action("Produce the direct result")]
            |done| -> result { 3 };

            #[choice("Widen the continuation")]
            #[case("Wide left")]
            #[case("Wide middle")]
            #[case("Wide right")]
            |selected| -> (wide_left, wide_middle, wide_right) {
                match selected { 0 => (), 1 => (), _ => () }
            };

            #[action("Build wide left")]
            |wide_left| -> result { 4 };

            #[action("Build wide middle")]
            |wide_middle| -> result { 5 };

            #[action("Build wide right")]
            |wide_right| -> result { 6 };

            #[end]
            |result| {};
        }
    "#;

    let scene = scene(source, "partial");
    let continuation_right = [Block(4), Block(5), Block(6), Block(7)]
        .into_iter()
        .map(|id| node(&scene, id).x)
        .max()
        .expect("the continuation is drawn");
    let early_terminal = scene
        .edges
        .iter()
        .find(|edge| edge.from == Block(3) && edge.to == Block(8))
        .expect("the early terminal reaches End");

    assert_eq!(node(&scene, Block(3)).x, skewer_x(3));
    assert!(node(&scene, Block(3)).x > continuation_right);
    assert_eq!(early_terminal.points[0].x, skewer_x(3));
}

#[test]
fn question_keeps_the_first_output_vertical_and_the_second_to_the_right() {
    let function: ItemFn = parse_quote! {
        fn decide(condition: bool) -> u8 {
            #[question("Choose a path")]
            |condition| -> (accepted, rejected) { condition };

            #[action("Produce the accepted result")]
            |accepted| -> result { 1 };

            #[action("Produce the rejected result")]
            |rejected| -> result { 0 };

            #[end]
            |result| {};
        }
    };
    let graph = kaalang_model::build(&function).expect("the flow is valid");
    let scene = layout(&graph, "decide(condition: bool) -> u8");
    let question = scene
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Question)
        .expect("the question is drawn");
    let edges = scene
        .edges
        .iter()
        .filter(|edge| edge.from == NodeId::Block(0))
        .collect::<Vec<_>>();

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].handover, ["accepted"]);
    assert_eq!(edges[0].points[0].x, question.x);
    assert_eq!(edges[0].points[0].y, question.y + question.height / 2);
    assert_eq!(edges[1].handover, ["rejected"]);
    assert_eq!(edges[1].points[0].x, question.x + question.width / 2);
    assert_eq!(edges[1].points[0].y, question.y);
    assert!(edges[1].points[1].x > question.x);
}

/// A hand-over leaving a question's right vertex drops its label level
/// with that question, so the label must clear the node it left: nodes
/// are drawn last and would paint over its first column.
#[test]
fn a_label_below_a_right_exit_clears_the_node_it_left() {
    let scene = scene(
        r#"
            #[kaalang]
            fn probe(condition: bool, extra: u8) {
                #[question("Does it need the long path?")]
                |condition| -> (other, a_long_result_wire_name_that_needs_room) {
                    condition
                };

                #[action("Work out the answer.")]
                |other| -> a_long_result_wire_name_that_needs_room {};

                #[end]
                |a_long_result_wire_name_that_needs_room, extra| {};
            }
        "#,
        "probe",
    );

    let question = scene
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::Question)
        .expect("the question is drawn");
    let label = scene
        .labels
        .iter()
        .find(|label| (label.at.y - question.y).abs() < question.height / 2)
        .expect("the right exit drops its label level with the question");
    assert!(label.lines.len() > 1, "the label must wrap to reach back");

    let left = label.at.x - label_width(&label.lines) / 2 - EDGE_LABEL_HALO;
    let right = question.x + question.width / 2;
    assert!(
        left >= right,
        "a label starting at x={left} runs back over a node ending at x={right}"
    );
}

/// End's capture label stacks above End inside the gap the collector row
/// occupies. The collector carries the other branches into End, so the
/// label's halo must not be painted over it.
#[test]
fn the_collector_row_clears_the_end_capture_label() {
    let scene = scene(
        r#"
            #[kaalang]
            fn halo(condition: bool) {
                #[action("Build the shared values.")]
                |condition| -> (
                    gate,
                    first_shared_wire_name,
                    second_shared_wire_name,
                    third_shared_wire_name
                ) { (condition, 1, 2, 3) };

                #[question("Which depth does this take?")]
                |gate| -> (short, long) { gate };

                #[action("Build the short result.")]
                |short| -> result {};

                #[action("Prepare the long result.")]
                |long| -> prepared {};

                #[action("Build the long result.")]
                |prepared| -> result {};

                #[end]
                |result, first_shared_wire_name, second_shared_wire_name, third_shared_wire_name| {};
            }
        "#,
        "halo",
    );

    let end = scene
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .expect("End is drawn");
    let capture = scene
        .labels
        .iter()
        .find(|label| {
            label
                .lines
                .concat()
                .starts_with("result, first_shared_wire_name")
        })
        .expect("End captures every wire that reaches it");
    assert!(capture.lines.len() > 2, "the capture label must wrap");
    // `at.y` is the first baseline, so the ink starts half a line above it.
    let label_top = capture.at.y - EDGE_LABEL_FONT / 2 - EDGE_LABEL_HALO;

    for edge in scene.edges.iter().filter(|edge| edge.to == end.id) {
        for segment in edge.points.windows(2) {
            if segment[0].y != segment[1].y {
                continue;
            }
            let row = segment[0].y;
            assert!(
                row < label_top,
                "a collector row at y={row} is under the capture label at y={label_top}"
            );
        }
    }
}

/// A terminal leaving a question's right vertex first joins the skewer
/// assigned to its branch, then descends without passing through later nodes.
#[test]
fn a_terminal_off_a_right_vertex_joins_its_assigned_skewer() {
    use NodeId::Block;

    let scene = scene(
        r#"
            #[kaalang]
            fn rex(condition: bool) {
                #[question("Is the short answer enough?")]
                |condition| -> (more, done) { condition };

                #[action("Work out the answer.")]
                |more| -> done {};

                #[end]
                |done| {};
            }
        "#,
        "rex",
    );

    let terminal = scene
        .edges
        .iter()
        .find(|edge| edge.from == Block(0) && edge.to == Block(2))
        .expect("the right output reaches End");
    assert_eq!(terminal.points[0].y, terminal.points[1].y);
    assert_eq!(terminal.points[1].x, skewer_x(1));

    for segment in terminal.points.windows(2) {
        for node in &scene.nodes {
            if node.id != terminal.from && node.id != terminal.to {
                assert!(
                    !enters_node(segment, node),
                    "the terminal crosses {:?}: {:?}",
                    node.id,
                    terminal.points
                );
            }
        }
    }
}

/// `segments_cross` compares a vertical run against a horizontal one, so
/// collinear overlap is out of scope. Two overlaps are deliberate: the
/// connections from a Select share the distributor row, and the connections
/// into End share the collector row and its descent.
#[test]
fn connections_are_orthogonal_and_free_of_perpendicular_crossings() {
    let source = r#"
#[kaalang]
fn route(request: u8) -> u8 {
#[question("Is there an application, and is it eligible?")]
|&request| -> (accepted, rejected) { request > 0 };

#[choice("Which path should process this application?")]
#[case("Short path")]
#[case("Long path with an additional check")]
|accepted, &request| -> (short, long) {
    match request {
        1 => (),
        _ => (),
    }
};

#[action("Prepare the short result.")]
|short, &request| -> selected { request };

#[action("Prepare the long result while preserving every important application detail.")]
|long, &request| -> selected { request };

#[action("Use the selected result.")]
|selected| -> result { selected };

#[action("Reject the application.")]
|rejected, request| -> result { request };

#[end]
|result| {};
}
"#;

    let scene = scene(source, "route");
    assert_no_unrelated_crossings(&scene);
}

fn assert_no_unrelated_crossings(scene: &Scene) {
    for edge in &scene.edges {
        assert!(
            edge.points
                .windows(2)
                .all(|points| points[0].x == points[1].x || points[0].y == points[1].y)
        );
    }

    for (index, left) in scene.edges.iter().enumerate() {
        for right in &scene.edges[index + 1..] {
            if left.from == right.from || left.to == right.to {
                continue;
            }
            assert!(!left.points.windows(2).any(|left| {
                right
                    .points
                    .windows(2)
                    .any(|right| segments_cross(left[0], left[1], right[0], right[1]))
            }));
        }
    }
}

/// The two continuing branches occupy skewers 0 and 1. Their four-skewer
/// continuation reserves 0 through 3, so the trailing terminal branches start
/// at 4 and 5 and no unrelated connections cross.
#[test]
fn a_wide_continuation_reserves_offsets_before_trailing_terminals() {
    let source = r#"
#[kaalang]
fn crossing(request: u8) -> u8 {
    #[choice("Outer.")]
    #[case("Left.")]
    #[case("Right.")]
    #[case("First terminal.")]
    #[case("Second terminal.")]
    |request| -> (left, right, first, second) {
        match request {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Left step.")]
    |left| -> shared { 1u8 };

    #[action("Right step.")]
    |right| -> shared { 2u8 };

    #[action("The first terminal ends one step in.")]
    |first| -> result { 3u8 };

    #[action("The second terminal takes one step more.")]
    |second| -> stepped { 4u8 };

    #[action("And ends deeper than the first.")]
    |stepped| -> result { stepped };

    #[choice("Inner, wide enough to block both terminal columns.")]
    #[case("A.")]
    #[case("B.")]
    #[case("C.")]
    #[case("D.")]
    |shared| -> (a, b, c, d) {
        match shared {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("A step.")]
    |a| -> selected { 6u8 };

    #[action("B step.")]
    |b| -> selected { 7u8 };

    #[action("C step.")]
    |c| -> selected { 8u8 };

    #[action("D step.")]
    |d| -> selected { 9u8 };

    #[action("Use the selected result.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}
"#;

    let scene = scene(source, "crossing");
    let offsets = [1, 2, 3, 4]
        .map(|index| node(&scene, NodeId::Block(index)).x)
        .map(|x| (x - skewer_x(0)) / (skewer_x(1) - skewer_x(0)));
    assert_eq!(offsets, [0, 1, 4, 5]);
    assert_eq!(node(&scene, NodeId::Block(5)).x, skewer_x(5));
    assert_no_unrelated_crossings(&scene);
}

/// The first continuing case reaches its yield inside a nested question, so
/// the shared continuation is drawn one skewer right of that case's own. The
/// reserved footprint is measured from where the continuation actually lands,
/// which is what keeps the trailing terminal clear of it.
#[test]
fn a_nested_arrival_moves_a_trailing_terminal_past_the_continuation() {
    use NodeId::Block;

    let source = r#"
#[kaalang]
fn nested(request: u8) -> u8 {
    #[choice("Choose an outer path.")]
    #[case("Take the nested path.")]
    #[case("Take the direct path.")]
    #[case("Reach End without joining.")]
    |request| -> (nested, direct, done) {
        match request {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Take one step down the nested path.")]
    |nested| -> stepped { true };

    #[question("Does the nested path end early?")]
    |stepped| -> (early, late) { stepped };

    #[action("Produce the early result.")]
    |early| -> result { 1u8 };

    #[action("Build the shared value on the late output.")]
    |late| -> shared { 2u8 };

    #[action("Build the shared value on the direct path.")]
    |direct| -> shared { 3u8 };

    #[choice("Select one of four shared results.")]
    #[case("Build A.")]
    #[case("Build B.")]
    #[case("Build C.")]
    #[case("Build D.")]
    |shared| -> (a, b, c, d) {
        match shared {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Build result A.")]
    |a| -> selected { 4u8 };

    #[action("Build result B.")]
    |b| -> selected { 5u8 };

    #[action("Build result C.")]
    |c| -> selected { 6u8 };

    #[action("Build result D.")]
    |d| -> selected { 7u8 };

    #[action("Use the selected result.")]
    |selected| -> result { selected };

    #[action("Produce the terminal result.")]
    |done| -> result { 8u8 };

    #[end]
    |result| {};
}
"#;

    let scene = scene(source, "nested");
    // The nested question yields on skewer 1, so the four-skewer continuation
    // reserves 1 through 4 and the trailing terminal starts at 5.
    assert_eq!(node(&scene, Block(11)).x, skewer_x(1));
    assert_eq!(node(&scene, Block(12)).x, skewer_x(5));
    let terminal = scene
        .edges
        .iter()
        .find(|edge| edge.from == Block(12) && edge.to == Block(13))
        .expect("the trailing terminal reaches End");

    for segment in terminal.points.windows(2) {
        for node in &scene.nodes {
            if node.id != terminal.from && node.id != terminal.to {
                assert!(
                    !enters_node(segment, node),
                    "the terminal crosses {:?}: {:?}",
                    node.id,
                    terminal.points
                );
            }
        }
    }
}

/// Reports whether an axis-aligned segment passes through a node's interior.
/// Anchors sit on the boundary, so only a strict crossing counts.
fn enters_node(segment: &[Point], node: &Node) -> bool {
    let (left, right) = (node.x - node.width / 2, node.x + node.width / 2);
    let (top, bottom) = (node.y - node.height / 2, node.y + node.height / 2);
    let [first, second] = segment else {
        return false;
    };
    if first.x == second.x {
        left < first.x
            && first.x < right
            && first.y.min(second.y) < bottom
            && top < first.y.max(second.y)
    } else {
        top < first.y
            && first.y < bottom
            && first.x.min(second.x) < right
            && left < first.x.max(second.x)
    }
}

fn segments_cross(first: Point, second: Point, third: Point, fourth: Point) -> bool {
    let (vertical_start, vertical_end, horizontal_start, horizontal_end) =
        if first.x == second.x && third.y == fourth.y {
            (first, second, third, fourth)
        } else if first.y == second.y && third.x == fourth.x {
            (third, fourth, first, second)
        } else {
            return false;
        };
    let horizontal_min = horizontal_start.x.min(horizontal_end.x);
    let horizontal_max = horizontal_start.x.max(horizontal_end.x);
    let vertical_min = vertical_start.y.min(vertical_end.y);
    let vertical_max = vertical_start.y.max(vertical_end.y);

    horizontal_min < vertical_start.x
        && vertical_start.x < horizontal_max
        && vertical_min < horizontal_start.y
        && horizontal_start.y < vertical_max
}
