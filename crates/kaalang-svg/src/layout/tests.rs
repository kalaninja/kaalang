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
fn terminal_cases_start_beyond_the_whole_shared_brancher() {
    let (source, flow) = fixture!("wire/behavior", "blocked_terminal_crossing");
    let file = crate::parse_file(source).unwrap();
    let function = crate::select_flow(&file.items, flow).unwrap();
    let model = kaalang_model::build(function).unwrap();
    let topology = topology::project(&model, "example", "-> u8");
    let placement = place::place(&topology, &model, &BTreeMap::new()).unwrap();
    let case = |choice, branch| placement.column(Vertex::Node(NodeId::Case { choice, branch }));
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
    assert_eq!(scene.topology.junctions[1].wires, ["result"]);
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

        let name = scene.topology.junctions[junction].wires.join(", ");
        let labels = scene
            .labels
            .iter()
            .filter(|label| label.lines == [name.clone()])
            .collect::<Vec<_>>();
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
                |condition| -> (yes, no) { condition };
                #[action("Build the yes value.")]
                |yes| -> #outputs { todo!() };
                #[action("Build the no value.")]
                |no| -> value { String::new() };
                #[action("Measure the value.")]
                |&value| -> result { value.len() };
            }
        };
        let model = kaalang_model::build(&function).unwrap();
        let scene = layout(&model, "example", "-> usize").unwrap();
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
            assert_eq!(scene.topology.junctions[0].wires, ["selected"]);
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
    let junction = scene
        .topology
        .junctions
        .iter()
        .position(|junction| junction.wires == ["false_result"])
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
    let file = crate::parse_file(source).expect("the fixture is valid Rust");
    let function = crate::select_flow(&file.items, flow).expect("the fixture declares the flow");
    let model = kaalang_model::build(function).expect("the fixture is a valid flow");
    let signature = signature_text(source, &function.sig);
    let scene = layout(
        &model,
        &signature,
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
            end.y > start.y,
            "{flow}: a connection does not reach a lower row"
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
        || -> first { 2u8 };

        #[action("Produce the second value.")]
        || -> second { 3u8 };

        #[action("Add the values.")]
        |&first, &second| -> sum { first + second };

        #[action("Multiply the values.")]
        |&first, &second| -> product { first * second };

        #[action("Finish from both combinations.")]
        |sum, product| -> result { sum + product };
    }
"#;

/// RFC 0003 §2: disjoint convergence groups of one brancher receive disjoint
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
    assert!(scene.topology.capture(NodeId::Block(1)).is_empty());
    assert_eq!(scene.topology.capture_label(NodeId::Block(1)), ["()"]);
    for consumer in [2, 3] {
        assert_eq!(
            scene.topology.capture(NodeId::Block(consumer)),
            ["&first", "&second"]
        );
    }
}

#[test]
fn independent_entry_blocks_continue_on_the_main_column_before_a_question() {
    let scene = drawn(fixture!("wire/behavior", "question_after_one_entry_block"));
    assert_main_sequence(&scene, &[3, 4, 5]);
    assert_eq!(
        scene.topology.handover(ExitId::of(NodeId::Block(3))),
        ["first"]
    );
    assert_eq!(scene.topology.capture(NodeId::Block(4)), ["right"]);
    assert_eq!(scene.topology.capture(NodeId::Block(5)), ["first"]);
}

#[test]
fn a_branch_effect_reaches_the_merge_before_common_work() {
    let scene = drawn(fixture!("wire/behavior", "effect_then_common"));
    let topology = &scene.topology;
    let effect = ExitId::of(NodeId::Block(2));
    let stamp = Vertex::Node(NodeId::Block(4));
    assert_eq!(topology.junctions.len(), 1);
    assert_eq!(topology.junctions[0].wires, ["value"]);
    assert!(topology.handover(effect).is_empty());
    assert_eq!(
        topology.leaving(effect).copied().collect::<Vec<_>>(),
        [crate::topology::Connection {
            source: Source::Exit(effect),
            destination: Destination::Junction(0),
        }]
    );
    assert_eq!(
        topology.incoming(stamp).copied().collect::<Vec<_>>(),
        [crate::topology::Connection {
            source: Source::Junction(0),
            destination: stamp,
        }]
    );
    assert_eq!(topology.capture_label(NodeId::Block(4)), ["()"]);
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
    assert!(scene.topology.capture(setup.id).is_empty());
    assert_eq!(scene.topology.handover(ExitId::of(setup.id)), ["setup"]);
    assert_eq!(scene.topology.capture(question.id), ["condition"]);
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
            |condition| -> (the_rather_long_named_left_branch_wire, no) {{ condition }};
            #[action("Short action.")]
            |the_rather_long_named_left_branch_wire| -> result {{ 1 }};
            #[action({:?})]
            |no| -> result {{ 2 }};
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
            |condition| -> (yes, a_long_branch_name_that_wraps_several_times_above_its_horizontal_connection) { condition };
            #[action("Take the first branch.")]
            |yes| -> result { 1 };
            #[action("Take the second branch.")]
            |a_long_branch_name_that_wraps_several_times_above_its_horizontal_connection| -> result { 2 };
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
            |condition| -> (fallback, proceed) { condition };
            #[action("Use the fallback.")]
            |fallback| -> result { 0 };
            #[action("Proceed.")]
            |proceed| -> result { 1 };
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
        scene.topology.handover(ExitId::of(NodeId::Start)),
        ["_value"]
    );
    assert_eq!(scene.topology.leaving(ExitId::of(NodeId::Start)).count(), 1);
    assert!(scene.topology.capture(NodeId::Block(0)).is_empty());
    assert_eq!(scene.topology.capture_label(NodeId::Block(0)), ["()"]);
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
            |first_seed| -> first { first_seed };

            #[action("Produce the second value.")]
            |second_seed| -> second { second_seed };

            #[action("Produce the third value.")]
            |third_seed| -> third { third_seed };

            #[action("Combine the values one way.")]
            |&first, &second, &third| -> one { first + second + third };

            #[action("Combine the values another way.")]
            |&first, &second, &third| -> two { first * second * third };

            #[action("Combine the values a third way.")]
            |&first, &second, &third| -> three { first ^ second ^ third };

            #[action("Finish from all three combinations.")]
            |one, two, three| -> result { one + two + three };
        }
    "#;
    let scene = drawn((source, "serial"));
    assert_main_sequence(&scene, &[0, 1, 2, 3, 4, 5, 6, 7]);
    for consumer in [3, 4, 5] {
        assert_eq!(
            scene.topology.capture(NodeId::Block(consumer)),
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
fn the_end_node_is_captioned_with_the_flow_return_type() {
    // A declared return type verbatim, and `-> ()` when the flow declares none.
    for ((source, flow), caption) in [
        (
            fixture!("end/behavior", "order_the_result_wire"),
            "-> (u32, u32, u32)",
        ),
        (fixture!("empty_flow/behavior", "nothing"), "-> ()"),
    ] {
        let scene = drawn((source, flow));
        let end = end_node(&scene);
        assert_eq!(scene.topology.node(end).label, caption, "{flow}");
        // The capture stays on the connection entering end, so the caption
        // names the type and the connection names the wire. It is drawn, not
        // only owned: neither flow merges `result`, so no junction label
        // stands in for it.
        assert_eq!(scene.topology.capture(end), ["result"], "{flow}");
        assert!(scene.topology.junctions.is_empty(), "{flow}");
        assert_eq!(
            scene
                .labels
                .iter()
                .filter(|label| label.lines == ["result"])
                .count(),
            1,
            "{flow}: the result capture is not drawn once at end"
        );
    }
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
    use unicode_segmentation::UnicodeSegmentation;

    for fields in [1, 12, 56] {
        let return_type = std::iter::repeat_n("[u8; 1]", fields)
            .collect::<Vec<_>>()
            .join(", ");
        let source =
            format!("#[kaalang] fn capsule(result: ({return_type})) -> ({return_type}) {{}}");
        let scene = drawn((&source, "capsule"));
        for id in [NodeId::Start, end_node(&scene)] {
            let node = scene.node(id);
            let ry = f64::from(node.height / 2);
            let rx = ry.min(f64::from(node.width / 2));
            let straight = f64::from(node.width / 2) - rx;
            let first_baseline = 15 - node.lines.len() as i32 * LINE_HEIGHT / 2;
            for (index, line) in node.lines.iter().enumerate() {
                let width = text::text_width(&line.graphemes(true).collect::<Vec<_>>(), LABEL_FONT);
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
