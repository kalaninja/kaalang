use super::*;

/// The source of `crates/kaalang/tests/<dir>/<stem>.rs` and the flow named after it.
macro_rules! fixture {
    ($dir:literal, $stem:literal) => {
        (
            include_str!(concat!("../../../kaalang/tests/", $dir, "/", $stem, ".rs")),
            $stem,
        )
    };
}

#[test]
fn a_loop_entry_fits_above_its_first_node_on_a_sibling_row() {
    let scene = drawn(fixture!("loop/behavior", "diverging_middle_branch"));
    let spin = named_node(&scene, "Spin.");
    assert_eq!(spin.y, named_node(&scene, "Advance.").y);
    let incoming = scene
        .connections
        .iter()
        .find(|edge| edge.destination == Destination::Node(spin.id))
        .unwrap();
    assert_eq!(
        Scene::bounds(spin).1 - incoming.points[0].y,
        vertical_gap(&scene),
        "the return arrow still leaves the usual gap above the body"
    );
}

#[test]
fn a_question_in_a_sibling_case_needs_no_extra_row() {
    let scene = drawn(fixture!("loop/behavior", "diverging_middle_branch"));
    assert_eq!(
        named_node(&scene, "Stay in the loop?").y,
        named_node(&scene, "Advance.").y,
        "independent first blocks below the same case row should align"
    );
}

#[test]
fn case_routes_share_one_distributor_rail() {
    for fixture in [
        fixture!("loop/behavior", "diverging_middle_branch"),
        fixture!("loop/behavior", "terminal_cases_after_repeats"),
        fixture!("choice/behavior", "run_choice"),
    ] {
        let scene = drawn(fixture);
        for select in scene
            .topology
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Select)
        {
            let routes = scene
                .connections
                .iter()
                .filter(|wire| {
                    wire.source == Source::Exit(ExitId::of(select.id))
                        && matches!(wire.destination, Destination::Node(NodeId::Case { .. }))
                })
                .collect::<Vec<_>>();
            let rails = routes
                .iter()
                .flat_map(|wire| wire.points.windows(2))
                .filter(|pair| pair[0].y == pair[1].y && pair[0].x != pair[1].x)
                .map(|pair| pair[0].y)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                rails.len(),
                1,
                "{}: the distributor split into separate rails",
                fixture.1
            );
            for route in routes {
                let straight = matches!(
                    route.destination,
                    Destination::Node(NodeId::Case { branch: 0, .. })
                );
                assert_eq!(
                    route.points.len(),
                    if straight { 2 } else { 4 },
                    "{}: the distributor has extra bends",
                    fixture.1
                );
            }
        }
    }
}

#[test]
fn routing_lanes_do_not_spend_node_columns() {
    let fixture = fixture!("loop/behavior", "diverging_middle_branch");
    let scene = drawn(fixture);
    let cases = scene
        .nodes
        .iter()
        .filter(|node| matches!(node.id, NodeId::Case { choice: 1, .. }))
        .collect::<Vec<_>>();
    for pair in cases.windows(2) {
        assert_eq!(
            pair[1].x - pair[0].x,
            COLUMN_WIDTH,
            "{}: empty routing columns separate the cases",
            fixture.1
        );
    }
    assert!(
        scene.width <= cases.len() as i32 * COLUMN_WIDTH + NODE_WIDTH,
        "{}: the detour spends a node width on each rail",
        fixture.1
    );
    for edge in &scene.connections {
        let side_exit =
            matches!(edge.source, Source::Exit(ExitId {branch: Some(branch), ..}) if branch > 0);
        assert!(
            edge.points.len() <= 6 + usize::from(side_exit),
            "{}: a route still has a staircase: {:?}",
            fixture.1,
            edge.points
        );
    }
    let start = scene
        .connections
        .iter()
        .find(|edge| {
            matches!(
                edge.source,
                Source::Exit(ExitId {
                    node: NodeId::Start,
                    ..
                })
            )
        })
        .unwrap();
    assert_eq!(
        start.points.len(),
        2,
        "{}: the entry still has a pocket",
        fixture.1
    );
}

#[test]
fn all_cases_of_a_choice_share_a_row() {
    for fixture in [
        fixture!("loop/behavior", "diverging_middle_branch"),
        fixture!("loop/behavior", "terminal_cases_after_repeats"),
        fixture!("loop/behavior", "outer_repeat_contour"),
    ] {
        let mut scene = drawn(fixture);
        let rows = scene
            .nodes
            .iter()
            .filter_map(|node| matches!(node.id, NodeId::Case { choice: 1, .. }).then_some(node.y))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            rows.len(),
            1,
            "{}: the cases must share their row",
            fixture.1
        );
        scene
            .nodes
            .iter_mut()
            .find(|node| {
                matches!(
                    node.id,
                    NodeId::Case {
                        choice: 1,
                        branch: 1
                    }
                )
            })
            .unwrap()
            .y += LANE;
        assert_eq!(
            route::verify(&scene),
            Some("choice 2 draws its cases on different rows".to_owned())
        );
    }
}

#[test]
fn terminal_cases_start_beyond_the_whole_shared_brancher() {
    let (source, flow) = fixture!("wire/behavior", "blocked_terminal_crossing");
    let file = crate::parse_file(source).unwrap();
    let function = crate::select_flow(&file.items, flow).unwrap();
    let model = kaalang_model::build(function).unwrap();
    let arrangement = &model.arrangement;
    let case = |choice, branch| arrangement.column[&Vertex::Node(NodeId::Case { choice, branch })];
    assert!(
        case(0, 2) > case(6, 3),
        "the first terminal case overlaps the inner choice"
    );
    assert!(case(0, 3) > case(0, 2));

    let scene = drawn((source, flow));
    let inner_right = scene
        .node(NodeId::Case {
            choice: 6,
            branch: 3,
        })
        .x;
    for branch in [2, 3] {
        assert!(scene.node(NodeId::Case { choice: 0, branch }).x > inner_right);
    }
}

#[test]
fn side_routes_end_horizontally_at_the_merge() {
    for (source, flow) in FIXTURES.into_iter().chain([fixture!(
        "wire/behavior",
        "nested_branch_passes_a_question_join"
    )]) {
        let scene = drawn((source, flow));
        for incoming in &scene.connections {
            let Destination::Junction(junction) = incoming.destination else {
                continue;
            };
            let points = &incoming.points;
            let end = points.last().unwrap();
            for outgoing in scene
                .connections
                .iter()
                .filter(|connection| connection.source == Source::Junction(junction))
            {
                assert_eq!(outgoing.points.first(), Some(end));
            }
            if points
                .windows(2)
                .any(|segment| segment[0].x != segment[1].x)
            {
                assert_eq!(
                    points[points.len() - 2].y,
                    end.y,
                    "{flow}: a side route turns down before its merge"
                );
            }
        }
    }
}

#[test]
fn sequential_questions_leave_sideways_and_merge_on_the_main_column() {
    let scene = drawn(fixture!(
        "wire/behavior",
        "a_branch_captures_a_merged_value"
    ));
    let main_x = scene.node(NodeId::Start).x;

    for block in [0, 3] {
        let node = scene.node(NodeId::Block(block));
        assert_eq!(node.x, main_x);
        for branch in 0..2 {
            let connection = scene
                .connections
                .iter()
                .find(|connection| {
                    connection.source
                        == Source::Exit(ExitId {
                            node: node.id,
                            branch: Some(branch),
                        })
                })
                .unwrap();
            let [start, next, ..] = connection.points[..] else {
                panic!("a question branch needs a route");
            };
            if branch == 0 {
                assert_eq!(
                    start,
                    Point {
                        x: node.x,
                        y: node.y + node.height / 2
                    }
                );
                assert_eq!(next.x, start.x);
                assert!(next.y > start.y);
            } else {
                assert_eq!(
                    start,
                    Point {
                        x: node.x + node.width / 2,
                        y: node.y
                    }
                );
                assert_eq!(next.y, start.y);
                assert!(next.x > start.x);
            }
        }
    }

    assert_eq!(scene.topology.junctions.len(), 2);
    assert_eq!(scene.captions.junction_wires(1), ["end"]);
    for junction in 0..2 {
        let common = scene
            .connections
            .iter()
            .find(|connection| connection.source == Source::Junction(junction))
            .unwrap();
        assert!(common.points.iter().all(|point| point.x == main_x));

        let merging = scene
            .connections
            .iter()
            .filter(|wire| wire.destination == Destination::Junction(junction))
            .collect::<Vec<_>>();
        let actions_bottom = merging.iter().map(|wire| wire.points[0].y).max().unwrap();
        let merge_line = merging
            .iter()
            .flat_map(|wire| wire.points.windows(2))
            .find(|segment| segment[0].y == segment[1].y)
            .unwrap();
        assert!(merge_line[0].y - actions_bottom >= MIN_VERTICAL_GAP / 2);

        let name = scene.captions.junction_wires(junction).join(", ");
        let labels = scene
            .labels
            .iter()
            .filter(|label| label.lines == [name.clone()])
            .collect::<Vec<_>>();
        if name == "end" {
            assert!(labels.is_empty(), "the end merge is labeled");
            continue;
        }
        assert_eq!(labels.len(), 1, "one shared label for {name}");
        assert!(labels[0].at.y > merge_line[0].y);
        assert!(labels[0].at.y < common.points.last().unwrap().y);
    }
}

#[test]
fn merge_labels_preserve_borrows_and_different_handovers() {
    for extra_output in [false, true] {
        let outputs: proc_macro2::TokenStream = if extra_output {
            syn::parse_quote!((value, _note))
        } else {
            syn::parse_quote!(value)
        };
        let function = syn::parse_quote! {
            fn borrowed_merge(condition: bool) -> usize {
                #[question("Choose a value.")]
                let (yes, no) = |condition| { condition };
                #[action("Build the yes value.")]
                let #outputs = |yes| { todo!() };
                #[action("Build the no value.")]
                let value = |no| { String::new() };
                #[action("Measure the value.")]
                let end = |&value| { value.len() };
            }
        };
        let model = kaalang_model::build(&function).unwrap();
        let scene = layout(&model, "example", &[], "usize").unwrap();
        let labels = scene
            .labels
            .iter()
            .map(|label| label.lines.join(" "))
            .collect::<Vec<_>>();
        assert_eq!(labels.iter().filter(|label| *label == "value").count(), 1);
        assert_eq!(labels.iter().filter(|label| *label == "&value").count(), 1);
        assert_eq!(labels.contains(&"value, _note".to_owned()), extra_output);
    }
}

#[test]
fn labels_clear_vertical_connections_and_routes_use_free_departure_columns() {
    for (source, name) in [
        fixture!("wire/behavior", "a_branch_captures_a_merged_value"),
        fixture!("wire/behavior", "nested_convergence"),
    ] {
        let scene = drawn((source, name));

        for label in &scene.labels {
            let (left, top, right, bottom) = label_rect(label);
            for segment in scene
                .connections
                .iter()
                .flat_map(|wire| wire.points.windows(2))
            {
                let [a, b] = [segment[0], segment[1]];
                if a.x == b.x && a.y.min(b.y) < bottom && a.y.max(b.y) > top {
                    assert!(
                        a.x < left || a.x > right,
                        "{name}: {:?} overlaps a vertical connection",
                        label.lines
                    );
                }
            }
        }

        if name == "nested_convergence" {
            let direct = scene.node(NodeId::Block(4));
            let route = scene
                .connections
                .iter()
                .find(|wire| wire.source == Source::Exit(ExitId::of(direct.id)))
                .unwrap();
            assert_eq!(
                route.points.len(),
                3,
                "descend, then finish horizontally at the merge"
            );
            assert_eq!(route.points[0].x, route.points[1].x);
            assert!(route.points.iter().all(|point| point.x <= direct.x));

            assert_eq!(scene.topology.junctions.len(), 1);
            assert_eq!(scene.captions.junction_wires(0), ["selected"]);
            let incoming = scene
                .connections
                .iter()
                .filter(|wire| wire.destination == Destination::Junction(0))
                .collect::<Vec<_>>();
            assert_eq!(incoming.len(), 3);
            let rail_heights = incoming
                .iter()
                .flat_map(|wire| wire.points.windows(2))
                .filter(|segment| segment[0].y == segment[1].y)
                .map(|segment| segment[0].y)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(rail_heights.len(), 1, "all three inputs meet on one rail");
            assert_eq!(
                scene
                    .labels
                    .iter()
                    .filter(|label| label.lines == ["selected"])
                    .count(),
                1
            );
        }
    }
}

#[test]
fn labels_beside_one_connection_align_by_their_left_edge() {
    let scene = drawn(fixture!("choice/behavior", "run_choice"));
    let left = |text: &str| {
        label_rect(
            scene
                .labels
                .iter()
                .find(|label| label.lines == [text])
                .unwrap(),
        )
        .0
    };

    assert_eq!(left("value, branch_action_count"), left("value"));
}

#[test]
fn nested_question_side_branches_share_their_merge_column() {
    let scene = drawn(fixture!("gallery/logical_formulas", "and"));
    let junction = (0..scene.topology.junctions.len())
        .find(|&junction| scene.captions.junction_wires(junction) == ["false_result"])
        .unwrap();
    let rail = scene.node(NodeId::Start).x + COLUMN_WIDTH;
    let incoming = scene
        .connections
        .iter()
        .filter(|connection| connection.destination == Destination::Junction(junction))
        .collect::<Vec<_>>();

    assert_eq!(incoming.len(), 3);
    assert!(
        incoming
            .iter()
            .all(|connection| connection.points[1..].iter().all(|point| point.x == rail))
    );
    assert_eq!(
        scene.node(NodeId::Block(3)).y,
        scene.node(NodeId::Block(4)).y
    );
}

/// Lays out one flow and holds it to RFC 0002 §8 before returning it, so every
/// test built on this helper carries the whole spatial contract with it.
fn drawn((source, flow): (&str, &str)) -> Scene {
    drawn_with((source, flow), |_| {})
}

/// The same, over an arrangement a test has changed first. Only a change the
/// model's own check still accepts is a witness, so a test using this says
/// which one it made and why it conforms.
fn drawn_with(
    (source, flow): (&str, &str),
    change: impl FnOnce(&mut kaalang_model::Arrangement),
) -> Scene {
    let file = crate::parse_file(source).expect("the fixture is valid Rust");
    let function = crate::select_flow(&file.items, flow).expect("the fixture declares the flow");
    let mut model = kaalang_model::build(function).expect("the fixture is a valid flow");
    model.compact_arrangement();
    change(&mut model.arrangement);
    let model = model;
    let start = start_text(source, &function.sig);
    let parameters = parameter_text(source, &function.sig);
    let scene = layout(
        &model,
        &start,
        &parameters,
        &return_text(source, &function.sig.output),
    )
    .expect("the fixture has a conforming diagram");

    assert_eq!(
        route::verify(&scene),
        None,
        "{flow}: a connection breaks RFC 0002 §8"
    );
    assert_eq!(
        label::verify(&scene),
        None,
        "{flow}: a label breaks RFC 0002 §8"
    );
    for connection in &scene.connections {
        let [start, .., end] = connection.points[..] else {
            panic!("{flow}: a routed connection has at least two points");
        };
        assert!(
            if scene.is_back_edge(connection) {
                end.y < start.y
            } else if matches!(connection.destination, Destination::Junction(_)) {
                // A side exit may meet a junction on the same horizontal run.
                end.y >= start.y
            } else {
                end.y > start.y
            },
            "{flow}: a connection does not follow its forward or return direction"
        );
    }

    scene
}

/// The fixtures whose shapes exercise the routing rules: branches, nested
/// branches, wire merges, separate roots and ordinary joins, plus
/// `question_after_a_partial_merge`, whose partial merge joins inside a wider
/// one.
const FIXTURES: [(&str, &str); 17] = [
    fixture!("wire/behavior", "blocked_terminal_crossing"),
    fixture!("wire/behavior", "closure_before_a_branch"),
    fixture!("wire/behavior", "effect_before_a_nested_terminal_branch"),
    fixture!("wire/behavior", "question_after_a_partial_merge"),
    fixture!("wire/behavior", "question_after_one_entry_block"),
    fixture!("wire/behavior", "shared_setup"),
    (SHARED_INPUTS, "shared_inputs"),
    fixture!("wire/behavior", "a_branch_captures_a_merged_value"),
    fixture!("wire/behavior", "independent_questions"),
    fixture!("wire/behavior", "independent_entry_blocks"),
    fixture!("wire/behavior", "local_work_before_a_wire_merge"),
    fixture!("wire/behavior", "nested_convergence"),
    fixture!("wire/behavior", "staged_convergence"),
    fixture!("wire/behavior", "blocks_without_shared_wires"),
    fixture!("choice/behavior", "run_choice"),
    fixture!("wire/behavior", "two_convergence_groups"),
    fixture!("wire/behavior", "two_merges_reach_one_consumer"),
];

const SHARED_INPUTS: &str = r#"
    #[kaalang]
    fn shared_inputs() -> u8 {
        #[action("Produce the first value.")]
        let first = || { 2 };

        #[action("Produce the second value.")]
        let second = || { 3 };

        #[action("Add the values.")]
        let sum = |&first, &second| { first + second };

        #[action("Multiply the values.")]
        let product = |&first, &second| { first * second };

        #[action("Finish from both combinations.")]
        let end = |sum, product| { sum + product };
    }
"#;

/// RFC 0003 §2.5: disjoint convergence groups of one brancher receive disjoint
/// footprints in authored branch order. `two_convergence_groups` shares one
/// continuation between its first two cases and another between its last two,
/// so nothing of the late half may sit at or left of the early half.
#[test]
fn disjoint_convergence_groups_take_disjoint_footprints() {
    let scene = drawn(fixture!("wire/behavior", "two_convergence_groups"));
    let x = |id| scene.node(id).x;
    let early = [
        NodeId::Case {
            choice: 0,
            branch: 0,
        },
        NodeId::Case {
            choice: 0,
            branch: 1,
        },
        NodeId::Block(1),
        NodeId::Block(2),
        NodeId::Block(3),
    ];
    let late = [
        NodeId::Case {
            choice: 0,
            branch: 2,
        },
        NodeId::Case {
            choice: 0,
            branch: 3,
        },
        NodeId::Block(4),
        NodeId::Block(5),
        NodeId::Block(6),
    ];

    let early_right = early.into_iter().map(x).max().unwrap();
    let late_left = late.into_iter().map(x).min().unwrap();
    assert!(
        early_right < late_left,
        "the two footprints overlap: early ends at {early_right}, late starts at {late_left}"
    );
    // Each group keeps its own merge; neither continuation is shared.
    assert_eq!(scene.topology.junctions.len(), 3);
}

#[test]
fn independent_producers_and_their_consumers_form_one_sequence() {
    let scene = drawn((SHARED_INPUTS, "shared_inputs"));
    assert_main_sequence(&scene, &[0, 1, 2, 3, 4, 5]);
    assert!(scene.captions.capture(NodeId::Block(1)).is_empty());
    assert_eq!(scene.captions.capture_label(NodeId::Block(1)), ["()"]);
    for consumer in [2, 3] {
        assert_eq!(
            scene.captions.capture(NodeId::Block(consumer)),
            ["&first", "&second"]
        );
    }
}

#[test]
fn independent_entry_blocks_continue_on_the_main_column_before_a_question() {
    let scene = drawn(fixture!("wire/behavior", "question_after_one_entry_block"));
    assert_main_sequence(&scene, &[3, 4, 5]);
    assert_eq!(
        scene.captions.handover(ExitId::of(NodeId::Block(3))),
        ["first"]
    );
    assert_eq!(scene.captions.capture(NodeId::Block(4)), ["right"]);
    assert_eq!(scene.captions.capture(NodeId::Block(5)), ["first"]);
}

#[test]
fn a_branch_effect_reaches_the_merge_before_common_work() {
    let scene = drawn(fixture!("wire/behavior", "effect_then_common"));
    let topology = &scene.topology;
    let captions = &scene.captions;
    let effect = ExitId::of(NodeId::Block(2));
    let stamp = Vertex::Node(NodeId::Block(4));
    assert_eq!(topology.junctions.len(), 1);
    assert_eq!(captions.junction_wires(0), ["value"]);
    assert!(captions.handover(effect).is_empty());
    assert_eq!(
        topology.leaving(effect).copied().collect::<Vec<_>>(),
        [kaalang_model::topology::Connection {
            source: Source::Exit(effect),
            destination: Destination::Junction(0),
        }]
    );
    assert_eq!(
        topology.incoming(stamp).copied().collect::<Vec<_>>(),
        [kaalang_model::topology::Connection {
            source: Source::Junction(0),
            destination: stamp,
        }]
    );
    assert_eq!(captions.capture_label(NodeId::Block(4)), ["()"]);
}

/// A serial stretch stays on the happy path, with one straight connection
/// between each adjacent pair even when their hand-over and capture differ.
fn assert_main_sequence(scene: &Scene, blocks: &[usize]) {
    let main_x = scene.node(NodeId::Start).x;
    for &block in blocks {
        assert_eq!(scene.node(NodeId::Block(block)).x, main_x);
    }
    for pair in blocks.windows(2) {
        let source = Source::Exit(ExitId::of(NodeId::Block(pair[0])));
        let outgoing = scene
            .connections
            .iter()
            .filter(|connection| connection.source == source)
            .collect::<Vec<_>>();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(
            outgoing[0].destination,
            Destination::Node(NodeId::Block(pair[1]))
        );
        assert!(outgoing[0].points.iter().all(|point| point.x == main_x));
        assert!(scene.node(NodeId::Block(pair[0])).y < scene.node(NodeId::Block(pair[1])).y);
    }
}

#[test]
fn shared_setup_precedes_its_consumers_question_on_the_main_column() {
    let scene = drawn(fixture!("wire/behavior", "shared_setup"));
    let setup = scene.node(NodeId::Block(0));
    let question = scene.node(NodeId::Block(1));
    assert_eq!(setup.x, scene.node(NodeId::Start).x);
    assert_eq!(setup.x, question.x);
    assert!(setup.y < question.y);
    assert!(scene.captions.capture(setup.id).is_empty());
    assert_eq!(scene.captions.handover(ExitId::of(setup.id)), ["setup"]);
    assert_eq!(scene.captions.capture(question.id), ["condition"]);
    let outgoing = scene
        .connections
        .iter()
        .filter(|connection| connection.source == Source::Exit(ExitId::of(setup.id)))
        .collect::<Vec<_>>();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].destination, Destination::Node(question.id));
    assert!(outgoing[0].points.iter().all(|point| point.x == setup.x));
    let input = scene
        .connections
        .iter()
        .find(|connection| connection.source == Source::Exit(ExitId::of(NodeId::Start)))
        .unwrap();
    assert_eq!(input.destination, Destination::Node(setup.id));
    assert!(input.points.iter().all(|point| point.x == setup.x));
    assert_eq!(
        scene.topology.incoming(Vertex::Node(question.id)).count(),
        1
    );
    let empty = scene
        .labels
        .iter()
        .filter(|label| label.lines == ["()"])
        .collect::<Vec<_>>();
    assert_eq!(empty.len(), 1);
    assert!(empty[0].at.y < scene.top_anchor(setup.id).y);
    assert!(empty[0].at.y > input.points[0].y);
    for consumer in [2, 3] {
        assert_eq!(
            scene
                .topology
                .incoming(Vertex::Node(NodeId::Block(consumer)))
                .count(),
            1
        );
    }
}

/// `drawn` holds every fixture to RFC 0002 §8 already, connections and labels
/// both; this adds the one thing it does not check, that the canvas exists.
#[test]
fn every_drawn_shape_satisfies_the_spatial_contract() {
    for (source, flow) in FIXTURES {
        let scene = drawn((source, flow));
        assert!(
            scene.width > 0 && scene.height > 0,
            "{flow}: the canvas has no extent"
        );
    }
}

#[test]
fn long_wire_labels_clear_a_tall_neighbor() {
    let source = format!(
        r#"
        #[kaalang]
        fn example(condition: bool) -> u8 {{
            #[question("Choose a branch.")]
            let (the_rather_long_named_left_branch_wire, no) = |condition| {{ condition }};
            #[action("Short action.")]
            let end = |the_rather_long_named_left_branch_wire| {{ 1 }};
            #[action({:?})]
            let end = |no| {{ 2 }};
        }}
    "#,
        "A tall description.\n".repeat(16)
    );
    let scene = drawn((&source, "example"));
    assert!(scene.labels.iter().any(|label| label.lines.len() > 1));
}

#[test]
fn a_wrapped_question_label_stays_above_its_horizontal_run() {
    let scene = drawn((
        r#"
        #[kaalang]
        fn example(condition: bool) -> u8 {
            #[question("Choose a branch.")]
            let (yes, a_long_branch_name_that_wraps_several_times_above_its_horizontal_connection) = |condition| { condition };
            #[action("Take the first branch.")]
            let end = |yes| { 1 };
            #[action("Take the second branch.")]
            let end = |a_long_branch_name_that_wraps_several_times_above_its_horizontal_connection| { 2 };
        }
    "#,
        "example",
    ));
    let label = scene
        .labels
        .iter()
        .find(|label| label.lines.len() > 1)
        .unwrap();
    let branch = scene
        .connections
        .iter()
        .find(|connection| {
            connection.source
                == Source::Exit(ExitId {
                    node: NodeId::Block(0),
                    branch: Some(1),
                })
        })
        .unwrap();
    assert!(label_rect(label).3 <= branch.points[0].y);
}

#[test]
fn a_right_question_branch_description_replaces_its_output_above_the_connection() {
    let scene = drawn((
        r#"
        #[kaalang]
        fn example(condition: bool) -> u8 {
            #[question("Choose a branch.")]
            #[no]
            #[yes("Take the longer continuation description that must wrap beside the branch.")]
            let (fallback, proceed) = |condition| { condition };
            #[action("Use the fallback.")]
            let end = |fallback| { 0 };
            #[action("Proceed.")]
            let end = |proceed| { 1 };
        }
    "#,
        "example",
    ));
    let question = scene.node(NodeId::Block(0));
    assert_eq!(scene.node(NodeId::Block(1)).x, question.x);
    assert!(scene.node(NodeId::Block(2)).x > question.x);

    let branch_labels = scene
        .labels
        .iter()
        .filter(|label| matches!(label.kind, LabelKind::Branch))
        .collect::<Vec<_>>();
    assert_eq!(branch_labels.len(), 1);
    assert!(branch_labels[0].lines.len() > 1);
    assert!(scene.labels.iter().all(|label| label.lines != ["proceed"]));
    assert!(
        scene
            .labels
            .iter()
            .any(|label| { matches!(label.kind, LabelKind::Wire) && label.lines == ["fallback"] })
    );
    let connection = scene
        .connections
        .iter()
        .find(|connection| {
            connection.source
                == Source::Exit(ExitId {
                    node: NodeId::Block(0),
                    branch: Some(1),
                })
        })
        .unwrap();
    assert!(label_rect(branch_labels[0]).3 <= connection.points[0].y);
}

#[test]
fn an_unused_hand_over_stays_visible_above_an_empty_capture() {
    // The transit connection does not make the action capture `_value`.
    let scene = drawn(fixture!("empty_flow/behavior", "discard_named"));
    assert_eq!(
        scene.captions.handover(ExitId::of(NodeId::Start)),
        ["_value"]
    );
    assert_eq!(scene.topology.leaving(ExitId::of(NodeId::Start)).count(), 1);
    assert!(scene.captions.capture(NodeId::Block(0)).is_empty());
    assert_eq!(scene.captions.capture_label(NodeId::Block(0)), ["()"]);
    assert!(
        scene.labels.iter().any(|label| label.lines == ["_value"]),
        "the unused hand-over is not drawn"
    );
}

#[test]
fn the_same_flow_renders_to_the_same_bytes() {
    // Two independently built models in one process meet differently seeded
    // hashers, so equal bytes mean no hash order reached the layout.
    for (source, flow) in FIXTURES {
        let first = crate::render_source(source, flow).expect("the fixture renders");
        let second = crate::render_source(source, flow).expect("the fixture renders");
        assert_eq!(first, second, "{flow} does not render deterministically");
    }
}

#[test]
fn separate_roots_and_their_consumer_share_the_main_column() {
    let scene = drawn(fixture!("wire/behavior", "blocks_without_shared_wires"));
    assert_main_sequence(&scene, &[0, 1, 2, 3]);
}

#[test]
fn three_producers_and_three_consumers_render_as_a_sequence() {
    // K3,3 in the capture dependencies needs no visual crossings: every
    // producer and consumer executes in sequence, with captures kept intact.
    let source = r#"
        #[kaalang]
        fn serial(first_seed: u8, second_seed: u8, third_seed: u8) -> u8 {
            #[action("Produce the first value.")]
            let first = |first_seed| { first_seed };

            #[action("Produce the second value.")]
            let second = |second_seed| { second_seed };

            #[action("Produce the third value.")]
            let third = |third_seed| { third_seed };

            #[action("Combine the values one way.")]
            let one = |&first, &second, &third| { first + second + third };

            #[action("Combine the values another way.")]
            let two = |&first, &second, &third| { first * second * third };

            #[action("Combine the values a third way.")]
            let three = |&first, &second, &third| { first ^ second ^ third };

            #[action("Finish from all three combinations.")]
            let end = |one, two, three| { one + two + three };
        }
    "#;
    let scene = drawn((source, "serial"));
    assert_main_sequence(&scene, &[0, 1, 2, 3, 4, 5, 6, 7]);
    for consumer in [3, 4, 5] {
        assert_eq!(
            scene.captions.capture(NodeId::Block(consumer)),
            ["&first", "&second", "&third"]
        );
    }
    assert!(crate::render_source(source, "serial").is_ok());
}

#[test]
fn branches_run_left_to_right_from_their_branchers_own_column() {
    for (source, flow) in FIXTURES {
        let scene = drawn((source, flow));
        for node in &scene.topology.nodes {
            let NodeId::Block(block) = node.id else {
                continue;
            };
            // Every exit of one brancher, in authored branch order, and where
            // the first thing each branch reaches ended up.
            let mut branches = scene
                .topology
                .exits
                .iter()
                .filter_map(|exit| match exit.id {
                    ExitId {
                        node: NodeId::Block(owner),
                        branch: Some(branch),
                    } if owner == block => Some(branch),
                    _ => None,
                })
                .filter_map(|branch| {
                    let exit = ExitId {
                        node: node.id,
                        branch: Some(branch),
                    };
                    let column = scene
                        .topology
                        .leaving(exit)
                        .filter_map(|connection| match connection.destination {
                            Destination::Node(reached) => Some(scene.node(reached).x),
                            Destination::Junction(_) => None,
                        })
                        .min()?;
                    Some((branch, column))
                })
                .collect::<Vec<_>>();
            branches.sort_unstable();
            if branches.len() < 2 {
                continue;
            }

            // RFC 0002 §8: the first output continues down the brancher's own
            // column and the rest appear to its right, in authored order.
            let own = scene.node(node.id).x;
            assert!(
                branches[0].1 >= own,
                "{flow}: the first branch of {block} runs left of its own column"
            );
            for pair in branches.windows(2) {
                assert!(
                    pair[1].1 > pair[0].1,
                    "{flow}: branch {} of {block} is not right of branch {}",
                    pair[1].0,
                    pair[0].0
                );
            }
        }
    }
}

/// The implicit end block's node, which `Flow::blocks` carries last.
fn end_node(scene: &Scene) -> NodeId {
    scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .expect("every flow has an end node")
        .id
}

#[test]
fn the_end_node_names_only_the_flow_return_type() {
    // A declared return type verbatim, and `()` when the flow declares none.
    for ((source, flow), caption) in [
        (
            fixture!("end/behavior", "order_the_end_wire"),
            "(u32, u32, u32)",
        ),
        (fixture!("empty_flow/behavior", "nothing"), "()"),
        (fixture!("end/behavior", "capture_from_a_branch"), "()"),
        (
            fixture!("capture/behavior", "local_mutability_of_outputs"),
            "(String, u32, u32)",
        ),
    ] {
        let scene = drawn((source, flow));
        let end = end_node(&scene);
        assert_eq!(scene.captions.label(end), caption, "{flow}");
        // The semantic capture remains available to the accessible description,
        // but the terminal route makes the `end` wire visually obvious.
        assert_eq!(scene.captions.capture(end), ["end"], "{flow}");
        assert!(
            scene.labels.iter().all(|label| !label
                .lines
                .join(" ")
                .split_whitespace()
                .any(|word| word == "end")),
            "{flow}: the end wire is labeled"
        );
    }
}

#[test]
fn result_is_labeled_as_an_ordinary_wire() {
    let scene = drawn(fixture!("end/behavior", "result_is_an_ordinary_wire"));
    assert!(scene.labels.iter().any(|label| label.lines == ["result"]));
}

#[test]
fn start_separates_the_flow_name_and_typed_parameters() {
    let source = r#"
        #[kaalang]
        fn example<'a, T>(r#type: &'a T, _: usize) -> &'a T
        where
            T: Copy,
        {
            #[action("Return the input.")]
            let end = |r#type| { r#type };
        }
    "#;
    let scene = drawn((source, "example"));
    assert_eq!(
        scene.captions.label(NodeId::Start),
        "example<'a, T> where T: Copy,"
    );
    let parameters = scene.parameters.as_ref().expect("the flow has parameters");
    assert_eq!(parameters.parameters, ["r#type: &'a T", "_: usize"]);
    assert_eq!(parameters.lines, parameters.parameters);
    assert!(parameters.x > scene.node(NodeId::Start).x);
}

#[test]
fn a_long_return_type_wraps_inside_the_end_node() {
    let scene = drawn(fixture!("end/behavior", "wrap_a_long_return_type"));
    let end = scene.node(end_node(&scene));
    assert!(
        end.lines.len() > 1,
        "a long return type does not wrap: {:?}",
        end.lines
    );
    // Sized from its own caption rather than a fixed height.
    assert!(
        end.height >= 30 + end.lines.len() as i32 * LINE_HEIGHT,
        "the end node does not grow to fit its caption"
    );
}

#[test]
fn capsule_captions_fit_the_curved_outline() {
    for fields in [1, 12, 56] {
        let return_type = std::iter::repeat_n("[u8; 1]", fields)
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!("#[kaalang] fn capsule(end: ({return_type})) -> ({return_type}) {{}}");
        let scene = drawn((&source, "capsule"));
        for id in [NodeId::Start, end_node(&scene)] {
            let node = scene.node(id);
            let ry = f64::from(node.height / 2);
            let rx = ry.min(f64::from(node.width / 2));
            let straight = f64::from(node.width / 2) - rx;
            let first_baseline = 15 - node.lines.len() as i32 * LINE_HEIGHT / 2;
            for (index, line) in node.lines.iter().enumerate() {
                let width = text::text_width(line, LABEL_FONT);
                let x = (f64::from(width) / 2.0 - straight).max(0.0);
                let baseline = first_baseline + index as i32 * LINE_HEIGHT;
                for y in [baseline - LABEL_FONT, baseline + LABEL_FONT / 3] {
                    let inside = (x / rx).powi(2) + (f64::from(y) / ry).powi(2) <= 1.0;
                    assert!(inside, "{id:?}: caption leaves the curved outline: {line}");
                }
            }
        }
    }
}

#[test]
fn a_terminal_exit_starts_beside_the_loop_body() {
    for (fixture, body, continuation) in [
        (
            fixture!("loop/behavior", "count_to"),
            "Increment the counter.",
            "Return the counter.",
        ),
        (
            fixture!("loop/behavior", "early_end_then_loop"),
            "Increment the count.",
            "Return the count.",
        ),
        (
            (
                include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs"),
                "binary_search",
            ),
            "Find the middle index.",
            "The target is absent.",
        ),
        (
            (
                include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs"),
                "binary_search_swapped",
            ),
            "Find the middle index.",
            "The target is absent.",
        ),
    ] {
        let scene = drawn(fixture);
        let body = named_node(&scene, body);
        let continuation = named_node(&scene, continuation);
        assert_ne!(continuation.x, body.x);
        assert_eq!(continuation.y, body.y, "{}", fixture.1);
    }
}

#[test]
fn a_nonterminal_loop_exit_starts_beside_the_body() {
    let (source, flow) = fixture!("loop/behavior", "nested_search");
    for following in [
        "",
        r#"#[action("Observe the next row.")] |&column, row| { let _ = row; };"#,
    ] {
        let source = source.replace(
            "|&column, &mut row| *row += 1;",
            &format!("|&column, &mut row| *row += 1; {following}"),
        );
        let scene = drawn((&source, flow));
        let body = named_node(&scene, "Does the value differ from the target?");
        let continuation = named_node(&scene, "Advance to the next row.");
        assert!(continuation.x > body.x);
        assert_eq!(continuation.y, body.y);
        if !following.is_empty() {
            let next = named_node(&scene, "Observe the next row.");
            assert_eq!(next.x, continuation.x);
            assert_eq!(next.y, named_node(&scene, "Advance to the next column.").y);
        }
    }
}

#[test]
fn a_following_loop_starts_beside_the_preceding_body() {
    let scene = drawn(fixture!("loop/behavior", "collect_steps"));
    let first = scene.topology.loops[0];
    let second = scene.topology.loops[1];
    let entry = scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(second.entry))
        .unwrap()
        .points[0];
    assert!(entry.x > scene.node(NodeId::Block(first.header + 1)).x);
    assert!(entry.y < named_node(&scene, "Build the first step.").y);
    assert!(
        scene.node(NodeId::Block(second.header + 1)).y
            <= named_node(&scene, "Record the first step.").y
    );
}

#[test]
fn a_single_side_exit_turns_up_without_descending_to_the_iteration_tail() {
    let fixture = fixture!("loop/behavior", "reversed_empty_loop");
    let scene = drawn(fixture);
    let loop_ = &scene.topology.loops[0];
    let incoming = scene
        .connections
        .iter()
        .filter(|edge| edge.destination == Destination::Junction(loop_.tail))
        .collect::<Vec<_>>();
    assert_eq!(incoming.len(), 1);
    let incoming = incoming[0];
    let departure = incoming.points[0];
    assert!(
        incoming.points.iter().all(|point| point.y == departure.y),
        "{}: the side exit should reach the return horizontally",
        fixture.1
    );
    let returning = scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(loop_.tail))
        .unwrap();
    assert_eq!(incoming.points.last(), returning.points.first());
    assert!(
        returning
            .points
            .windows(2)
            .all(|segment| segment[1].y <= segment[0].y)
    );
}

#[test]
fn a_single_action_returns_after_the_usual_gap() {
    let scene = drawn(fixture!("loop/behavior", "nested_search"));
    let outer = &scene.topology.loops[0];
    let action = named_node(&scene, "Advance to the next row.");
    let returning = scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(outer.tail))
        .unwrap();
    let start = returning.points[0];
    assert_eq!(start.x, action.x);
    assert_eq!(
        start.y - Scene::bounds(action).3,
        vertical_gap(&scene),
        "the return should turn beside the action instead of below the inner body"
    );
    let incoming = scene
        .connections
        .iter()
        .find(|edge| edge.destination == Destination::Junction(outer.tail))
        .unwrap();
    assert_eq!(incoming.points.last(), Some(&start));
}

#[test]
fn terminal_routes_align_with_independent_returns_and_clear_crossing_returns() {
    let binary_search = include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs");
    let early_end = fixture!("loop/behavior", "early_end_then_loop");
    // Wrapping makes this action two pixels taller than its repeating sibling.
    let wrapped_end = early_end.0.replace(
        "Return the count.",
        "Return the counter after the loop finishes.",
    );
    for fixture in [
        fixture!("loop/behavior", "count_to"),
        fixture!("loop/behavior", "collect_steps"),
        early_end,
        (wrapped_end.as_str(), early_end.1),
        fixture!("loop/behavior", "nested_search"),
        (binary_search, "binary_search"),
        (binary_search, "binary_search_swapped"),
    ] {
        let scene = drawn(fixture);
        let end = scene
            .topology
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::End)
            .unwrap();
        let connection = scene
            .connections
            .iter()
            .find(|edge| edge.destination == Destination::Node(end.id))
            .unwrap();
        let terminal_y = match connection.source {
            Source::Junction(_) => connection.points[0].y,
            Source::Exit(_) => connection.points.last().unwrap().y,
        };
        let lowest_return = scene
            .connections
            .iter()
            .filter(|edge| scene.is_back_edge(edge))
            .flat_map(|edge| &edge.points)
            .map(|point| point.y)
            .max()
            .unwrap();
        let gap = vertical_gap(&scene);
        assert!(
            terminal_y >= lowest_return,
            "{}: the terminal route clears the returns",
            fixture.1
        );
        if matches!(
            fixture.1,
            "count_to" | "binary_search" | "binary_search_swapped"
        ) {
            assert_eq!(
                terminal_y, lowest_return,
                "{}: independent terminal and return routes need no extra row",
                fixture.1
            );
        }
        if matches!(connection.source, Source::Junction(_)) {
            assert_eq!(
                scene.top_anchor(end.id).y - terminal_y,
                gap,
                "{}",
                fixture.1
            );
        }
    }
}

#[test]
fn distributors_and_loop_tails_leave_the_usual_gap() {
    let source = include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs");
    for flow in ["binary_search", "binary_search_swapped"] {
        let scene = drawn((source, flow));
        let gap = vertical_gap(&scene);
        let tail = scene.topology.loops[0].tail;
        for incoming in scene
            .connections
            .iter()
            .filter(|edge| edge.destination == Destination::Junction(tail))
        {
            assert_eq!(
                incoming.points.last().unwrap().y - incoming.points[0].y,
                gap,
                "{flow}: iteration tail"
            );
        }
        for branch in [1, 2] {
            let connection = scene
                .connections
                .iter()
                .find(|edge| {
                    edge.destination == Destination::Node(NodeId::Case { choice: 5, branch })
                })
                .unwrap();
            let [start, turn, across, end] = connection.points[..] else {
                panic!("{flow}: the distributor should have two bends");
            };
            assert_eq!(turn.y - start.y, gap, "{flow}: below select");
            assert_eq!(end.y - across.y, gap, "{flow}: above case");
        }
    }
}

#[test]
fn loop_returns_are_explicit_and_forward_precedence_is_not_drawn() {
    for (fixture, returns) in [
        (
            (
                include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs"),
                "binary_search",
            ),
            1,
        ),
        (fixture!("loop/behavior", "count_to"), 1),
        (
            (
                include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs"),
                "binary_search_swapped",
            ),
            1,
        ),
        (fixture!("loop/behavior", "nested_search"), 2),
        (fixture!("loop/behavior", "collect_steps"), 2),
        (fixture!("loop/behavior", "condition_effects"), 1),
        (fixture!("loop/behavior", "reversed_empty_loop"), 1),
        (fixture!("loop/behavior", "nested_loop_tail"), 2),
        (fixture!("loop/behavior", "early_end_then_loop"), 1),
        (fixture!("loop/behavior", "first_value"), 0),
    ] {
        let scene = drawn(fixture);
        assert_eq!(scene.topology.loops.len(), returns);
        assert_eq!(
            scene
                .connections
                .iter()
                .filter(|edge| scene.is_back_edge(edge))
                .count(),
            returns
        );
        for edge in &scene.topology.order {
            assert!(
                !scene
                    .connections
                    .iter()
                    .any(|drawn| drawn.source == edge.source
                        && drawn.destination == edge.destination)
            );
        }
        for loop_ in &scene.topology.loops {
            let header = NodeId::Block(loop_.header + 1);
            let entry = Destination::Junction(loop_.entry);
            let edge = scene
                .connections
                .iter()
                .find(|edge| {
                    edge.source == Source::Junction(loop_.tail) && edge.destination == entry
                })
                .expect("every repeating body returns to the entry before its condition");
            let end = *edge.points.last().unwrap();
            let anchor = scene.top_anchor(header);
            assert_eq!(end.x, anchor.x);
            assert!(end.y < anchor.y, "the arrow stops above the question");
            assert_eq!(edge.points[edge.points.len() - 2].y, end.y);
            assert_ne!(edge.points[edge.points.len() - 2].x, end.x);
            let outgoing = scene
                .connections
                .iter()
                .find(|edge| edge.source == Source::Junction(loop_.entry))
                .expect("one shared segment leads from the entry to the condition");
            assert_eq!(outgoing.destination, Destination::Node(header));
            assert_eq!(outgoing.points, [end, anchor]);
            assert!(scene.connections.iter().any(|incoming| {
                incoming.destination == entry
                    && !scene.is_back_edge(incoming)
                    && incoming.points.last() == Some(&end)
            }));
            assert!(
                edge.points
                    .windows(2)
                    .any(|segment| segment[1].y < segment[0].y)
            );
        }
        let svg = crate::render_source(fixture.0, fixture.1).expect("the loop renders");
        assert_eq!(svg.matches("marker-end=").count(), returns);
        assert!(!svg.contains("__kaalang_scoped"));
    }
}

#[test]
fn a_loop_without_captures_or_a_return_adds_no_visual_space() {
    let body = r#"
        #[action("Initialize.")]
        let mut count = || 0;
        loop {
            #[question("Finished?")]
            let (done, again) = |&count, limit| *count == limit;
            #[action("Return the count.")]
            let end = |done, count| count;
            #[action("Advance.")]
            |again, &mut count| *count += 1;
        }
    "#;
    let plain = format!("#[kaalang] fn example(limit: usize) -> usize {{ {body} }}");
    let nested = format!("#[kaalang] fn example(limit: usize) -> usize {{ loop {{ {body} }} }}");
    assert_eq!(
        crate::render_source(&plain, "example").unwrap(),
        crate::render_source(&nested, "example").unwrap(),
    );
}

#[test]
fn an_empty_unconditional_loop_returns_on_the_left_without_an_end_node() {
    let fixture = fixture!("loop/behavior", "empty_loop");
    let scene = drawn(fixture);
    let loop_ = scene.topology.loops[0];
    assert!(
        !scene
            .topology
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::End)
    );
    let edge = scene
        .connections
        .iter()
        .find(|edge| scene.is_back_edge(edge))
        .expect("the empty loop returns to its entry");
    assert_eq!(edge.destination, Destination::Junction(loop_.entry));
    let entry = edge.points.last().expect("the return reaches its entry");
    assert!(edge.points.iter().any(|point| point.x < entry.x));

    let svg = crate::render_source(fixture.0, fixture.1).expect("the empty loop renders");
    assert_eq!(svg.matches("marker-end=").count(), 1);
    assert!(!svg.contains("class=\"node end\""));
}

#[test]
fn nested_loop_boundaries_render_with_and_without_end() {
    for (fixture, returns, ends) in [
        (fixture!("loop/behavior", "empty_trailing_loop"), 2, 0),
        (fixture!("loop/behavior", "conditional_nested_loop"), 0, 1),
    ] {
        let scene = drawn(fixture);
        assert_eq!(
            scene
                .connections
                .iter()
                .filter(|edge| scene.is_back_edge(edge))
                .count(),
            returns
        );
        assert_eq!(
            scene
                .topology
                .nodes
                .iter()
                .filter(|node| node.kind == NodeKind::End)
                .count(),
            ends
        );
        let svg =
            crate::render_source(fixture.0, fixture.1).expect("nested loop boundaries render");
        assert_eq!(svg.matches("marker-end=").count(), returns);
        assert_eq!(svg.matches("class=\"node end\"").count(), ends);
    }
}

/// A compacted return still clears every node of its body.
#[test]
fn loop_returns_clear_the_whole_body() {
    let fixture = fixture!("loop/behavior", "empty_trailing_loop");
    let scene = drawn(fixture);
    let outer = scene.topology.loops[0];
    let edge = scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(outer.tail))
        .unwrap();
    let contour = edge.points.iter().map(|point| point.x).max().unwrap();
    for node in &scene.nodes {
        if scene.bodies[0].contains(&Vertex::Node(node.id)) {
            let (_, _, right, _) = Scene::bounds(node);
            assert!(
                contour > right,
                "{}: the return climbs inside the body's {:?}",
                fixture.1,
                node.id
            );
        }
    }
    let arrival = scene
        .connections
        .iter()
        .find(|edge| edge.destination == Destination::Junction(outer.tail))
        .unwrap();
    assert_eq!(arrival.points.last(), edge.points.first());
}

#[test]
fn a_compact_side_return_clears_a_wrapped_branch_description() {
    let (source, flow) = fixture!("loop/behavior", "empty_trailing_loop");
    let description = "Restart the entire outer iteration after the inner loop finishes.";
    let source = source.replace("#[no(\"NO\")]", &format!("#[no(\"{description}\")]"));
    let scene = drawn((&source, flow));
    let label = scene
        .labels
        .iter()
        .find(|label| label.lines.concat() == description)
        .unwrap();
    assert!(label.lines.len() > 1);
    let outer = scene.topology.loops[0];
    let edge = scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(outer.tail))
        .unwrap();
    let vertical = edge
        .points
        .windows(2)
        .find(|segment| segment[1].y < segment[0].y)
        .unwrap();
    assert!(vertical[0].x >= label_rect(label).2 + LANE);
}

#[test]
fn an_end_enclosed_by_nested_returns_is_rejected() {
    let source =
        include_str!("../../../kaalang/tests/loop/compile_fail/end_enclosed_by_nested_returns.rs");
    let error = crate::render_source(source, "end_enclosed_by_nested_returns").unwrap_err();
    let crate::RenderError::InvalidFlow { message, .. } = &error else {
        panic!("an unrealizable topology is an invalid flow, not a rendering error: {error}")
    };
    // The realizability decision, not some earlier wire rule.
    assert!(
        message.contains("could not construct a diagram under RFC 0002"),
        "{message}"
    );
}

#[test]
fn end_sits_below_nested_returns_when_an_alternative_arrangement_allows_it() {
    let scene = drawn(fixture!("loop/behavior", "end_below_nested_returns"));
    let end = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .unwrap();
    let top = Scene::bounds(scene.node(end.id)).1;
    let lowest_return = scene
        .connections
        .iter()
        .filter(|edge| scene.is_back_edge(edge))
        .flat_map(|edge| &edge.points)
        .map(|point| point.y)
        .max()
        .unwrap();
    assert!(top > lowest_return, "end should finish the diagram");
}

/// The geometry check catches an end that is not last, whichever way it is
/// not last.
///
/// No flow reaches this: `kaalang_model` orders every other vertex before end
/// and checks the ranks, so the renderer only has to hold on to that through
/// compaction. The mutations stand in for a compaction that lets go of it.
/// A return driven inside the body it leaves is caught by the same gate the
/// compactions revert on.
///
/// `verify_returns` runs inside `route::verify`, so `compact_returns`,
/// `end::adjust` and `close_unused_rows` each undo a move that breaks the
/// contour rule instead of failing the whole drawing. Pulling one climb back
/// over its own body is what that gate has to see.
#[test]
fn a_return_inside_its_body_is_caught_by_the_geometry_check() {
    let fixture = fixture!("loop/behavior", "trailing_inner_loop");
    let scene = drawn(fixture);
    assert!(
        super::route::verify(&scene).is_none(),
        "the fixture itself keeps every return outside its body"
    );
    for index in 0..scene.topology.loops.len() {
        let mut pulled = drawn(fixture);
        let tail = pulled.topology.loops[index].tail;
        let back = pulled
            .connections
            .iter()
            .position(|edge| edge.source == Source::Junction(tail))
            .expect("every loop draws its return");
        // One lane back towards the body it leaves: still clear of every node
        // box, so only the contour rule can object.
        let inward = match pulled.arrangement.contours[index].side {
            Side::Left => super::LANE,
            Side::Right => -super::LANE,
        };
        for point in &mut pulled.connections[back].points[1..3] {
            point.x += inward;
        }
        assert_eq!(
            super::route::verify(&pulled).as_deref(),
            Some(
                format!(
                    "the return of the loop at block {} climbs inside its body",
                    pulled.topology.loops[index].header + 1
                )
                .as_str()
            ),
            "a return pulled back over its own body should be caught"
        );
    }
}

#[test]
fn a_disconnected_junction_is_caught_by_the_geometry_gates() {
    let mut scene = drawn(fixture!("loop/behavior", "empty_loop"));
    let entry = scene.topology.loops[0].entry;
    let incoming = scene
        .connections
        .iter_mut()
        .find(|edge| {
            edge.destination == Vertex::Junction(entry) && matches!(edge.source, Source::Exit(_))
        })
        .expect("the loop entry has an initial arrival");
    incoming
        .points
        .last_mut()
        .expect("the arrival has an endpoint")
        .y -= 1;

    // A gap creates no crossing. The routes must still meet at one point.
    let accepted_compaction = conforms(&mut scene);
    let accepted_final = finish(scene).is_ok();
    assert_eq!(
        (accepted_compaction, accepted_final),
        (false, false),
        "a disconnected junction must fail both compaction and final validation"
    );
}

#[test]
fn a_misplaced_end_is_caught_by_the_geometry_check() {
    let fixture = fixture!("loop/behavior", "end_below_nested_returns");
    let scene = drawn(fixture);
    assert!(
        super::end::verify(&scene).is_none(),
        "the fixture itself keeps end last"
    );
    let end = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .unwrap()
        .id;
    let reach = scene.height;

    // A node left below end.
    let mut raised = drawn(fixture);
    for node in &mut raised.nodes {
        if node.id == end {
            node.y -= reach;
        }
    }
    assert!(
        super::end::verify(&raised).is_some(),
        "a node below end should be caught"
    );
    // The rule is only kept through the compactions because the gate they
    // revert on runs it, so the wiring is part of what this pins.
    assert_eq!(
        super::route::verify(&raised).as_deref(),
        Some("end must be below every other node and loop return"),
        "the geometry gate should run the end rule"
    );

    // A return running down past it. Level with end's top edge is allowed —
    // the block still sits below the rail — so this has to go further.
    let mut trailing = drawn(fixture);
    let top = Scene::bounds(trailing.node(end)).1;
    let back = trailing
        .connections
        .iter()
        .position(|edge| trailing.is_back_edge(edge))
        .expect("the fixture has a return");
    assert!(
        {
            for point in &mut trailing.connections[back].points {
                point.y = top;
            }
            super::end::verify(&trailing).is_none()
        },
        "a return level with end's top edge is allowed"
    );
    for point in &mut trailing.connections[back].points {
        point.y = top + 1;
    }
    assert!(
        super::end::verify(&trailing).is_some(),
        "a return below end should be caught"
    );
}

fn named_node<'a>(scene: &'a Scene, description: &str) -> &'a Node {
    let id = scene
        .topology
        .nodes
        .iter()
        .find(|node| scene.captions.label(node.id) == description)
        .expect("the fixture contains the described node")
        .id;
    scene.node(id)
}

#[test]
fn loop_and_break_captures_are_unlabeled() {
    let scene = drawn(fixture!("loop/behavior", "entry_captures"));
    assert!(
        scene
            .topology
            .junctions
            .iter()
            .any(|junction| junction.is_break)
    );
    for capture in [
        "entry, mut ticket, &count, &mut log",
        "mut ticket, &log, &mut count",
    ] {
        assert!(
            !scene
                .labels
                .iter()
                .any(|label| label.lines.concat() == capture)
        );
    }
    assert!(scene.labels.iter().any(|label| label.lines == ["done"]));
    let selected = drawn(fixture!("loop/behavior", "conditional_entry"));
    assert!(
        !selected
            .topology
            .junctions
            .iter()
            .any(|junction| junction.is_break)
    );
    assert!(selected.labels.iter().any(|label| label.lines == ["done"]));
}

#[test]
fn an_enclosed_return_is_rejected_before_layout() {
    for source in [
        include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat.rs"),
        include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat_mixed_targets.rs"),
        include_str!("../../../kaalang/tests/loop/compile_fail/enclosed_repeat_nested.rs"),
    ] {
        assert!(matches!(
            crate::render_source(source, "invalid"),
            Err(crate::RenderError::InvalidFlow { message, .. })
                if message.contains("reorder the branches")
        ));
    }
}

#[test]
fn enclosed_cases_are_rejected_before_layout() {
    for (source, name) in [
        fixture!("loop/compile_fail", "enclosed_break"),
        fixture!("loop/compile_fail", "end_case_between_breaks"),
        fixture!("loop/compile_fail", "terminal_case_between_repeats"),
        fixture!("loop/compile_fail", "two_terminal_cases_between_repeats"),
    ] {
        let name = if name == "enclosed_break" {
            "invalid"
        } else {
            name
        };
        assert!(matches!(crate::render_source(source, name),
            Err(crate::RenderError::InvalidFlow { message, .. })
                if message.contains("no conforming arrangement")));
    }
}

/// A label reaching into the gap a return climbs is answered by widening the
/// columns, not by giving up on the return.
///
/// The label is wrapped to fit the standard gap beside its own column, and
/// the return of the loop in the next column climbs in that same gap. Stepping
/// the rail outward walks further into the label, and stepping it inward walks
/// into the body, so the only answer is more room between the columns — which
/// is only measurable once the labels are placed.
#[test]
fn a_label_reaching_into_a_return_gap_widens_the_columns() {
    let source = r#"
        #[kaalang]
        fn collect_steps(enabled: bool, limit: usize) -> Vec<String> {
            #[question("Collect the steps?")]
            let (run, skip) = |enabled| enabled;

            #[action("Start an empty log.")]
            let mut log = |run| Vec::new();

            |&log, &limit| loop {
                #[question("Are there more first steps?")]
                #[yes("YES")]
                #[no("NO")]
                let (iterate_1, leave_1) = |&log, limit| log.len() < limit;

                |leave_1| break;

                #[action("Build the first step.")]
                let wwwwwwwwwwwwwwwwwwwwwwww = |iterate_1| {
                    let mut text = String::new();
                    'native: for value in [0, 1, 2] {
                        if value == 0 {
                            continue 'native;
                        }
                        text.push('a');
                        break 'native;
                    }
                    text
                };

                #[action("Record the first step.")]
                |&mut log, wwwwwwwwwwwwwwwwwwwwwwww| log.push(wwwwwwwwwwwwwwwwwwwwwwww);
            };

            |&log, &limit| loop {
                #[question("Are there more second steps?")]
                #[yes("YES")]
                #[no("NO")]
                let (iterate_2, leave_2) = |&log, limit| log.len() < limit * 2;

                |leave_2| break;

                #[question("Is the log length odd?")]
                let (odd, even) = |iterate_2, &log| log.len() % 2 == 1;

                #[action("Build an odd step.")]
                let wwwwwwwwwwwwwwwwwwwwwwww = |odd| String::from("b");

                #[action("Build an even step.")]
                let wwwwwwwwwwwwwwwwwwwwwwww = |even| String::from("c");

                #[action("Record the second step.")]
                |&mut log, wwwwwwwwwwwwwwwwwwwwwwww| log.push(wwwwwwwwwwwwwwwwwwwwwwww);
            };

            #[action("Return the log.")]
            let end = |log| log;

            #[action("Return an empty log.")]
            let end = |skip| Vec::new();
        }
    "#;
    let scene = drawn((source, "collect_steps"));
    assert!(
        label::verify(&scene).is_none(),
        "no label may be struck by a return"
    );
    assert!(
        scene.column_x(1) - scene.column_x(0) > COLUMN_WIDTH,
        "the columns should have been widened for it"
    );
}

/// A return driven past the return of a loop nested in its body is caught by
/// the same gate.
///
/// The two spans need not overlap, so no crossing check sees them, and the
/// bodies they climb beside can be identical — which is why the contour rule
/// names the nested rails as well as the body's own vertices. Here the outer
/// return spans the rows above the inner loop and the inner return the rows
/// below it, so the two never share one.
#[test]
fn a_return_inside_a_nested_return_is_caught_by_the_geometry_check() {
    let source = r#"
        #[kaalang]
        fn nested_contours(mode: u8) -> u8 {
            loop {
                #[question("Repeat the outer loop?")]
                let (again, enter) = |mode| mode == 0;

                #[action("Repeat the outer loop.")]
                |again| ();

                #[action("Take the first step.")]
                let first = |enter| ();

                #[action("Take the second step.")]
                let second = |first| ();

                #[action("Take the third step.")]
                let third = |second| ();

                |third| loop {};
            }
        }
    "#;
    let scene = drawn((source, "nested_contours"));
    assert!(super::route::verify(&scene).is_none(), "the flow conforms");
    assert_eq!(scene.topology.loops.len(), 2, "an outer and an inner loop");

    // Bring the inner return round to the outer one's side and past it. Both
    // stay clear of every body, and their rows still never meet, so only the
    // rule that holds an enclosing return outside a nested one objects.
    let mut passed = drawn((source, "nested_contours"));
    let outer = rail(&passed, 0);
    passed.arrangement.contours[1].side = passed.arrangement.contours[0].side;
    let beyond = match passed.arrangement.contours[0].side {
        Side::Left => outer - LANE,
        Side::Right => outer + LANE,
    };
    let back = passed
        .connections
        .iter()
        .position(|edge| edge.source == Source::Junction(passed.topology.loops[1].tail))
        .expect("every loop draws its return");
    for point in &mut passed.connections[back].points[1..3] {
        point.x = beyond;
    }
    assert_eq!(
        super::route::verify(&passed).as_deref(),
        Some(
            format!(
                "the return of the loop at block {} climbs inside the return nested in its body at {beyond}",
                passed.topology.loops[0].header + 1
            )
            .as_str()
        ),
        "an enclosing return driven past a nested one should be caught"
    );
}

/// Where one loop's return climbs.
fn rail(scene: &Scene, index: usize) -> i32 {
    scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(scene.topology.loops[index].tail))
        .expect("every loop draws its return")
        .points[1]
        .x
}

/// A label reaching past a nested return moves the whole chain of contours
/// outside it, not just the one it strikes.
///
/// The inner rail cannot step alone: one lane out is where the rail enclosing
/// it climbs, and the contour rule refuses that. Widening the columns moves
/// the label and both rails together, so it never resolves either. What does
/// is stepping the enclosing rails with the one that has to move.
#[test]
fn a_label_past_a_nested_return_moves_the_chain_outside_it() {
    let source = r#"
        #[kaalang]
        fn nested_exit_convergence(mut count: usize) -> usize {
            loop {
                loop {
                    #[choice("Leave the inner loop?")]
                    #[case("Leave at zero.")]
                    #[case("Leave at one.")]
                    #[case("Count down.")]
                    let (zero, one, wwwwwwww) = |count| match count {
                        0 => (),
                        1 => (),
                        _ => (),
                    };

                    |zero| break;

                    |one| break;

                    #[action("Count down.")]
                    |wwwwwwww, &mut count| *count -= 1;
                }

                #[question("Finish the outer loop?")]
                let (done, wwwwwwww) = |count| count == 0;

                |done| break;

                #[action("Count down once more.")]
                |wwwwwwww, &mut count| *count -= 1;
            }

            #[action("Return the result.")]
            let end = |count| count;
        }
    "#;
    let scene = drawn((source, "nested_exit_convergence"));
    assert!(
        label::verify(&scene).is_none(),
        "no label may be struck by a return"
    );
    assert!(
        super::route::verify(&scene).is_none(),
        "and the rails must still clear each other"
    );
    let rails = (0..scene.topology.loops.len())
        .map(|index| rail(&scene, index))
        .collect::<Vec<_>>();
    assert_eq!(rails.len(), 2, "an outer and an inner loop");
    assert!(
        rails[0].abs_diff(rails[1]) >= LANE.unsigned_abs(),
        "the two rails keep their lane apart: {rails:?}"
    );
}

#[test]
fn translating_the_drawing_preserves_its_return_contours() {
    let mut scene = drawn(fixture!("loop/behavior", "nested_exit_convergence"));
    for node in &mut scene.nodes {
        node.x -= 2000;
    }
    for connection in &mut scene.connections {
        for point in &mut connection.points {
            point.x -= 2000;
        }
    }
    if let Some(parameters) = &mut scene.parameters {
        parameters.x -= 2000;
    }
    assert_eq!(route::verify(&scene), None);
}

#[test]
fn clearing_one_label_can_take_more_than_one_lane() {
    let mut scene = drawn(fixture!("loop/behavior", "empty_loop"));
    let back = scene
        .connections
        .iter()
        .find(|edge| scene.is_back_edge(edge))
        .unwrap();
    let climb = back
        .points
        .windows(2)
        .find(|pair| pair[0].y > pair[1].y)
        .unwrap();
    scene.labels = vec![Label {
        kind: LabelKind::Wire,
        lines: vec!["wwwwwwwwww".to_owned()],
        at: Point {
            x: climb[0].x - 3 * LANE,
            y: i32::midpoint(climb[0].y, climb[1].y),
        },
    }];
    assert_eq!(
        clear_labels(&mut scene),
        0,
        "there is unlimited room to the left"
    );
    scene.indent();
    scene.fit();
    assert_eq!(label::verify(&scene), None);
    assert_eq!(route::verify(&scene), None);
}

/// The probe `a_return_may_stand_beyond_the_body_it_clears` checks in the
/// model: a loop whose three cases repeat, repeat and leave.
const FAR_CONTOUR: (&str, &str) = (
    r#"
        #[kaalang]
        fn far_contour(mode: u8) -> u8 {
            loop {
                #[choice("Which route?")]
                #[case("Case 0 repeat.")]
                #[case("Case 1 repeat.")]
                #[case("Case 2 break.")]
                let (case_0, case_1, case_2) = |mode| match mode {
                    0 => (),
                    1 => (),
                    _ => (),
                };
                #[action("Advance in case 0.")]
                |case_0| ();
                #[action("Advance in case 1.")]
                |case_1| ();
                |case_2| break;
            }
            #[action("Return the mode.")]
            let end = |mode| mode;
        }
    "#,
    "far_contour",
);

/// A return standing further out than its body's boxes suggest is drawn where
/// the arrangement put it, not where the boxes would have put it.
///
/// The column is one of the three decisions a contour records, and the
/// renderer realizes it through the same column-to-pixel map every node and
/// route uses. Measuring the body alone would bring this rail two columns in
/// and quietly draw a diagram the model did not choose.
#[test]
fn a_return_beyond_its_body_is_drawn_where_the_arrangement_put_it() {
    let near = drawn(FAR_CONTOUR);
    let contour = near.arrangement.contours[0];
    let beyond = match contour.side {
        Side::Left => near.arrangement.column.values().copied().min(),
        Side::Right => near.arrangement.column.values().copied().max(),
    }
    .expect("the probe has columns")
        + match contour.side {
            Side::Left => -2,
            Side::Right => 2,
        };

    let far = drawn_with(FAR_CONTOUR, |arrangement| {
        arrangement.contours[0].column = beyond;
    });
    // `indent` slides the whole drawing, so the recorded column is read as an
    // offset from a column the drawing also holds. Start owns column 0.
    let anchor = far.node(NodeId::Start).x + far.column_x(beyond) - far.column_x(0);
    let drawn_at = rail(&far, 0);
    match contour.side {
        Side::Left => assert!(
            drawn_at <= anchor - LANE,
            "the rail should climb outside the recorded column: {drawn_at} against {anchor}"
        ),
        Side::Right => assert!(
            drawn_at >= anchor + LANE,
            "the rail should climb outside the recorded column: {drawn_at} against {anchor}"
        ),
    }
}

/// Unlike `FAR_CONTOUR`, this tail has one arrival, so `compact_returns`
/// shortens it. The model test checks the same farther contour as a witness.
#[test]
fn a_compacted_return_keeps_its_recorded_far_contour() {
    let scene = drawn_with(
        fixture!("loop/behavior", "reversed_empty_loop"),
        |arrangement| {
            assert_eq!(arrangement.contours[0].side, Side::Right);
            arrangement.contours[0].column = arrangement.column.values().max().unwrap() + 2;
        },
    );
    let tail = scene.topology.loops[0].tail;
    assert_eq!(
        scene
            .topology
            .connections
            .iter()
            .filter(|edge| edge.destination == Vertex::Junction(tail))
            .count(),
        1,
        "the witness must exercise sole-arrival compaction"
    );
    let anchor = scene.node(NodeId::Start).x + scene.column_x(scene.arrangement.contours[0].column)
        - scene.column_x(scene.column(Vertex::Node(NodeId::Start)));
    let drawn_at = rail(&scene, 0);
    assert!(
        drawn_at >= anchor + LANE,
        "the compacted return must stay outside its recorded column: {drawn_at} against {anchor}"
    );
}

/// Four loops nested one inside the next. The model's
/// `four_nested_returns_climb_four_lanes_on_one_side` checks the same witness
/// in abstract columns; this one draws it.
const FOUR_LANES: (&str, &str) = (
    r#"
        #[kaalang]
        fn deep(mut step: usize) -> usize {
            loop {
                #[question("Leave the first?")]
                let (stay_0, leave_0) = |&step| *step > 0;
                |leave_0| break;
                |stay_0| loop {
                    #[question("Leave the second?")]
                    let (stay_1, leave_1) = |&step| *step > 1;
                    |leave_1| break;
                    |stay_1| loop {
                        #[question("Leave the third?")]
                        let (stay_2, leave_2) = |&step| *step > 2;
                        |leave_2| break;
                        |stay_2| loop {
                            #[question("Leave the fourth?")]
                            let (stay_3, leave_3) = |&step| *step > 3;
                            |leave_3| break;
                            #[action("Advance at the deepest level.")]
                            |stay_3, &mut step| *step += 1;
                        };
                    };
                };
            }
            #[action("Return the step.")]
            let end = |step| step;
        }
    "#,
    "deep",
);

/// A witness whose outermost return climbs in lane 3 is drawn with all four
/// rails on one side, ordered outward with the nesting, and the outermost four
/// lanes clear of everything the body draws.
///
/// The lane bound is the loop count, so four mutually enclosing loops is where
/// it is tight, and no fixture reaches past lane 1. What this pins is the
/// drawing: four rails, each a lane outside the one it encloses, and the last
/// of them four lanes past the body. For loops nested this way the lane a
/// contour records is implied by the nesting, so the renderer would reach the
/// same rails by counting enclosed rails alone — the point is that it reaches
/// them at all, over a lane index nothing else in either crate draws.
#[test]
fn four_nested_returns_are_drawn_in_four_lanes() {
    let scene = drawn_with(FOUR_LANES, |arrangement| {
        let side = arrangement.contours[0].side;
        for (index, contour) in arrangement.contours.iter_mut().enumerate() {
            contour.side = side;
            contour.lane = 3 - index;
        }
    });
    assert_eq!(scene.topology.loops.len(), 4, "four nested loops");

    let side = scene.arrangement.contours[0].side;
    let rails = (0..4).map(|index| rail(&scene, index)).collect::<Vec<_>>();
    for (outer, inner) in rails.iter().zip(rails.iter().skip(1)) {
        let apart = match side {
            Side::Left => inner - outer,
            Side::Right => outer - inner,
        };
        assert!(
            apart >= LANE,
            "an enclosing rail should climb at least a lane outside the one it encloses: {rails:?}"
        );
    }

    let edge = scene
        .nodes
        .iter()
        .map(|node| match side {
            Side::Left => node.x - node.width / 2,
            Side::Right => node.x + node.width / 2,
        })
        .reduce(|edge, other| match side {
            Side::Left => edge.min(other),
            Side::Right => edge.max(other),
        })
        .expect("the probe draws nodes");
    assert!(
        (edge - rails[0]).abs() >= 4 * LANE,
        "the outermost rail should climb four lanes past the body: {} past {edge}",
        rails[0]
    );
}

#[path = "../../../kaalang/tests/support/diagram_shapes.rs"]
mod diagram_shapes;

/// Every generated loop shape the model accepts also renders.
///
/// The model decides realizability, so an accepted flow this renderer cannot
/// draw is a renderer defect rather than an authored one — `UnroutableTopology`
/// says as much. Nothing else looks for one: the fixture corpus is what people
/// wrote, and these are the shapes nobody writes by hand.
#[test]
fn every_generated_shape_the_model_accepts_also_renders() {
    let sources = diagram_shapes::loop_shapes()
        .into_iter()
        .chain(diagram_shapes::question_shapes())
        .map(|source| format!("#[kaalang]\n{source}"))
        .collect::<Vec<_>>();
    assert_eq!(sources.len(), 2835);

    let (mut drawn, mut refused) = (0, 0);
    for source in &sources {
        let file = crate::parse_file(source).expect("the probe is valid Rust");
        let function = crate::select_flow(&file.items, "probe").expect("the probe declares it");
        let model = match kaalang_model::build(function) {
            Ok(model) => model,
            Err(error) => {
                assert!(
                    !error.to_string().contains("internal kaalang"),
                    "{source}\n{error}"
                );
                refused += 1;
                continue;
            }
        };
        drawn += 1;
        let start = start_text(source, &function.sig);
        let parameters = parameter_text(source, &function.sig);
        let scene = layout(
            &model,
            &start,
            &parameters,
            &return_text(source, &function.sig.output),
        )
        .unwrap_or_else(|reason| panic!("{source}\nan accepted flow did not render: {reason}"));
        assert_eq!(route::verify(&scene), None, "{source}");
        assert_eq!(label::verify(&scene), None, "{source}");
        assert_eq!(correspondence(&scene), None, "{source}");
    }
    assert_eq!((drawn, refused), (1094, 1741));
}

/// The same bent witness is checked by the model's `a_return_can_bend_outside_its_body`.
#[test]
fn a_return_keeps_its_recorded_bend_and_clears_labels() {
    let mut scene = drawn_with(FAR_CONTOUR, |arrangement| {
        let contour = arrangement.contours[0];
        let delta = match contour.side {
            Side::Left => -1,
            Side::Right => 1,
        };
        let gap = arrangement.ranks / 2;
        let lane = arrangement.gap_lanes[gap];
        arrangement.gap_lanes[gap] += 1;
        arrangement.return_routes.insert(
            0,
            kaalang_model::Route {
                departure: contour.column,
                arrival: contour.column + delta,
                runs: vec![kaalang_model::Run {
                    gap,
                    lane,
                    enter: contour.column,
                    exit: contour.column + delta,
                }],
            },
        );
    });
    assert_eq!(correspondence(&scene), None);
    let back = scene
        .connections
        .iter()
        .position(|edge| scene.is_back_edge(edge))
        .unwrap();
    assert_eq!(scene.connections[back].points.len(), 6);
    scene.connections[back].points[1].x += 1;
    scene.connections[back].points[2].x += 1;
    assert!(
        correspondence(&scene)
            .unwrap()
            .contains("changed its recorded corridor")
    );
}

#[test]
fn a_return_clears_its_drawn_entry_and_tail() {
    let scene = drawn(fixture!("loop/behavior", "reversed_empty_loop"));
    for first in [false, true] {
        let mut moved = scene.clone();
        let index = moved
            .connections
            .iter()
            .position(|edge| moved.is_back_edge(edge))
            .unwrap();
        let points = &mut moved.connections[index].points;
        let endpoint = if first { 0 } else { points.len() - 1 };
        // This right return now stands left of its own endpoint. No other
        // body vertex moved, so the body-extents check alone cannot see it.
        points[endpoint].x = points[1].x + 1;
        assert!(
            route::verify(&moved)
                .unwrap()
                .contains("climbs inside its body")
        );
    }
}

#[test]
fn shortening_a_tail_also_closes_its_exclusive_contour_column() {
    for fixture in [
        fixture!("loop/behavior", "empty_trailing_loop"),
        fixture!("loop/behavior", "reversed_empty_loop"),
        fixture!("loop/behavior", "nested_loop_tail"),
    ] {
        let scene = drawn(fixture);
        let tail = scene.topology.loops[0].tail;
        assert_eq!(scene.arrangement.contours[0].side, Side::Right);
        let back = scene
            .connections
            .iter()
            .find(|edge| edge.source == Source::Junction(tail))
            .unwrap();
        assert_eq!(
            rail(&scene, 0),
            back.points[0].x + LANE,
            "{}: the shortened tail needs one contour lane, not an empty column",
            fixture.1
        );
        assert_eq!(correspondence(&scene), None);
    }
}
