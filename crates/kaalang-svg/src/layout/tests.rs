use super::*;
use std::rc::Rc;

use crate::captions::Captions;
use crate::text::joined;

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
fn a_shifted_cycle_entry_is_rejected_even_when_its_routes_still_meet() {
    let mut scene = drawn(fixture!("loop/behavior", "merged_break"));
    let entry = scene.topology.loops[0].entry;
    let gaps = vertical_gaps(&scene);
    let y = scene
        .rows(&gaps)
        .line_y(RunLine::Rank(scene.rank(Vertex::Junction(entry))));
    for point in scene
        .connections
        .iter_mut()
        .flat_map(|edge| &mut edge.points)
    {
        if point.y == y {
            point.y -= 1;
        }
    }
    assert_eq!(route::verify(&scene), None, "the shifted routes still meet");
    assert_eq!(
        correspondence(&scene),
        Some(format!(
            "junction {entry} is drawn away from its row centre"
        ))
    );
}

#[test]
fn a_tail_shifted_off_its_column_is_rejected() {
    let mut scene = drawn(fixture!("loop/behavior", "reversed_empty_loop"));
    let tail = scene.topology.loops[0].tail;
    for edge in &mut scene.connections {
        if edge.source == Source::Junction(tail) {
            edge.points.first_mut().unwrap().x += 5;
        }
        if edge.destination == Vertex::Junction(tail) {
            edge.points.last_mut().unwrap().x += 5;
        }
    }
    assert_eq!(
        route::verify(&scene),
        None,
        "the shifted tail still connects"
    );
    assert_eq!(label::verify(&scene), None);
    assert_eq!(loop_block::verify(&scene), None);
    assert_eq!(
        correspondence(&scene),
        Some(format!(
            "junction {tail} is drawn away from its column centre"
        ))
    );
}

#[test]
fn shared_wire_labels_stay_at_the_receiving_node() {
    let fixture = fixture!("loop/behavior", "conditional_entry");
    for repetitions in [1, 40] {
        let run = "run".repeat(repetitions);
        let skip = "skip".repeat(repetitions);
        let source = fixture.0.replace("run", &run).replace("skip", &skip);
        let scene = drawn((&source, fixture.1));
        for (name, caption) in [
            (run, "Initialize the selected counter."),
            (skip, "Skip the counter."),
        ] {
            let node = named_node(&scene, caption);
            let labels = scene
                .labels
                .iter()
                .filter(|label| joined(&label.lines, "") == name)
                .collect::<Vec<_>>();
            assert_eq!(labels.len(), 1, "one label represents both ends");
            assert_eq!(labels[0].owner, Vertex::Node(node.id));
            assert_eq!(label_rect(labels[0]).3, scene.top_anchor(node.id).y);
            assert!(labels[0].at.x > node.x);
        }
    }
}

#[test]
fn cycle_entry_labels_use_the_same_padding_when_shared() {
    let fixture = fixture!("loop/behavior", "conditional_nested_loop");
    for name in ["flag".to_owned(), "a_long_input_wire_name".repeat(8)] {
        let source = fixture.0.replace("flag", &name);
        let scene = drawn((&source, fixture.1));
        for (boundary, region) in scene
            .topology
            .loop_boundaries
            .iter()
            .zip(&scene.loop_regions)
        {
            let Vertex::Node(node) = boundary.entry else {
                panic!("both cycles enter their first body node directly");
            };
            let input = scene
                .labels
                .iter()
                .find(|label| label.owner == boundary.entry)
                .unwrap();
            assert!(label_rect(input).1 >= region.top);
            if input.lines.len() == 1 {
                assert_eq!(scene.top_anchor(node).y - region.top, 35);
            }
        }
        assert_eq!(
            scene
                .labels
                .iter()
                .filter(|label| joined(&label.lines, "") == name)
                .count(),
            1
        );
    }
}

#[test]
fn a_cycle_boundary_does_not_overlap_its_interface_rails() {
    let mut scene = drawn(fixture!("loop/behavior", "count_to"));
    let entry = scene.topology.loop_boundaries[0].entry;
    let entry = scene
        .connections
        .iter()
        .find(|edge| edge.destination == entry)
        .and_then(|edge| edge.points.last())
        .copied()
        .expect("the cycle has an entry interface");

    scene.loop_regions[0].top = entry.y;
    assert!(
        loop_block::verify(&scene).is_some_and(|reason| reason.contains("boundary overlaps route"))
    );
}

#[test]
fn long_cycle_captions_wrap_or_shorten_without_changing_geometry() {
    use unicode_segmentation::UnicodeSegmentation;

    let source = r#"
        #[kaalang]
        fn example(flag: bool) {
            #[question("Choose a cycle.")]
            let (left, right) = |flag| flag;
            #[cycle("Collect.")]
            let done = |left| {
                #[action("Collect the results.")]
                let ready = |left| ();
                |ready| break;
            };
            #[cycle("Skip.")]
            let done = |right| { |right| break; };
            |done| return;
        }
    "#;
    let reference = drawn((source, "example"));
    let unicode = "👩‍💻e\u{301}".repeat(64);
    for (description, shortened) in [
        ("Collect the results.", false),
        (
            "Collect the available results from the source until there are enough items to complete the current request.",
            true,
        ),
        (unicode.as_str(), true),
    ] {
        let changed = source
            .replace("Collect.", description)
            .replace("Skip.", description);
        let scene = drawn((&changed, "example"));
        assert_eq!(
            (scene.width, scene.height),
            (reference.width, reference.height)
        );
        for (region, before) in scene.loop_regions.iter().zip(&reference.loop_regions) {
            assert_eq!(
                (region.left, region.top, region.right, region.bottom),
                (before.left, before.top, before.right, before.bottom)
            );
            assert_eq!(region.description, description);
            assert!(region.caption.len() <= 2);
            let shown = joined(&region.caption, "");
            let prefix = shown.trim_end_matches('…');
            assert!(description.starts_with(prefix));
            assert!(
                prefix.len() == description.len()
                    || description
                        .grapheme_indices(true)
                        .any(|(index, _)| index == prefix.len())
            );
        }
        let caption = &scene.loop_regions[0].caption;
        assert_eq!(caption.len(), 2, "the body leaves room to wrap");
        if shortened {
            assert!(caption.last().unwrap().as_ref().ends_with('…'));
        } else {
            assert_eq!(joined(caption, ""), description);
        }
        assert!(
            scene.loop_regions[1].caption.is_empty(),
            "the empty cycle has no caption space"
        );
    }
}

#[test]
fn all_cases_of_a_choice_share_a_row() {
    for fixture in [
        fixture!("loop/behavior", "diverging_middle_branch"),
        fixture!("loop/behavior", "terminal_cases_after_repeats"),
        fixture!("loop/behavior", "outer_repeat_contour"),
    ] {
        let mut scene = drawn(fixture);
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
fn a_merge_side_route_cannot_join_above_its_marker() {
    let mut scene = drawn(fixture!("wire/behavior", "two_merges_reach_one_consumer"));
    let incoming = scene
        .connections
        .iter_mut()
        .find(|edge| {
            matches!(edge.destination, Destination::Junction(junction)
                if !scene.topology.junctions[junction].merges.is_empty())
                && edge.points.len() == 3
        })
        .expect("the fixture has a sideways merge arrival");
    let end = *incoming.points.last().unwrap();
    // Keep the endpoint, but join the other producer above the merge marker.
    incoming.points[1].y -= LANE;
    incoming.points.insert(
        2,
        Point {
            x: end.x,
            y: end.y - LANE,
        },
    );

    assert_eq!(
        route::verify(&scene).as_deref(),
        Some("a merge side route turns downward before its endpoint")
    );
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

                |end| return end;
            }
        };
        let model = kaalang_compiler::build(&function).unwrap();
        let captions = captions_of(&model, "example", "usize");
        let scene = layout(&model, &captions, &[]).unwrap();
        let labels = scene
            .labels
            .iter()
            .map(|label| joined(&label.lines, " "))
            .collect::<Vec<_>>();
        assert_eq!(labels.iter().filter(|label| *label == "value").count(), 1);
        assert_eq!(labels.iter().filter(|label| *label == "&value").count(), 1);
        assert_eq!(labels.contains(&"value, _note".to_owned()), extra_output);
    }
}

/// The captions one layout pass reads, derived once as the renderer does.
fn captions_of(model: &SemanticModel, start: &str, return_type: &str) -> Rc<Captions> {
    Rc::new(crate::captions::derive(model, start, return_type))
}

/// Lays out one flow and holds it to RFC 0002 §8 before returning it, so every
/// test built on this helper carries the whole spatial contract with it.
fn drawn((source, flow): (&str, &str)) -> Scene {
    drawn_with((source, flow), |_| {})
}

/// The same, over an arrangement a test has changed first. Only a change the
/// model's own check still accepts is a witness, so a test using this says
/// which one it made and why it conforms.
fn drawn_with((source, flow): (&str, &str), change: impl FnOnce(&mut Arrangement)) -> Scene {
    let file = crate::parse_file(source).expect("the fixture is valid Rust");
    let function = crate::select_flow(&file.items, flow).expect("the fixture declares the flow");
    let mut model = kaalang_compiler::build(&function).expect("the fixture is a valid flow");
    kaalang_render::compact_arrangement(&mut model);
    change(&mut model.arrangement);
    let model = model;
    let start = start_text(source, &function.sig);
    let parameters = parameter_text(source, &function.sig);
    let captions = captions_of(&model, &start, &return_text(source, &function.sig.output));
    let scene =
        layout(&model, &captions, &parameters).expect("the fixture has a conforming diagram");

    for connection in &scene.connections {
        let [start, .., end] = connection.points[..] else {
            panic!("{flow}: a routed connection has at least two points");
        };
        assert!(
            if scene.is_back_edge(connection) {
                end.y < start.y
            } else if matches!(connection.destination, Destination::Junction(_))
                || matches!(connection.source, Source::Junction(_))
            {
                // A junction connection may collapse to its attachment point.
                end.y >= start.y
            } else {
                end.y > start.y
            },
            "{flow}: a connection does not follow its forward or back edge direction"
        );
    }

    scene
}

/// Routing fixtures covering branches, merges, roots, joins, and call captions.
const FIXTURES: [(&str, &str); 18] = [
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
    fixture!("call/behavior", "call_inside_a_branch"),
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

        |end| return end;
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
    assert_main_sequence(&scene, &[0, 1, 2, 3, 4]);
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
        [kaalang_compiler::topology::Connection {
            source: Source::Exit(effect),
            destination: Destination::Junction(0),
        }]
    );
    assert_eq!(
        topology.incoming(stamp).copied().collect::<Vec<_>>(),
        [kaalang_compiler::topology::Connection {
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
        .filter(|label| joined(&label.lines, "") == "()")
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

            |end| return end;
        }}
    "#,
        "A tall description.\n".repeat(16)
    );
    let scene = drawn((&source, "example"));
    assert!(scene.labels.iter().any(|label| label.lines.len() > 1));
}

#[test]
fn long_labels_enlarge_only_their_adjacent_row_gaps() {
    let scene = drawn((
        r#"
        #[kaalang]
        fn example(condition: bool) -> u8 {
            #[question("Choose a branch.")]
            #[no("Take the deliberately long fallback description that wraps onto several lines.")]
            #[yes("Take the deliberately long continuation description that wraps onto several lines.")]
            let (fallback, proceed) = |condition| { condition };
            #[action("Use the fallback.")]
            let end = |fallback| { 0 };
            #[action("Proceed.")]
            let end = |proceed| { 1 };

            |end| return end;
        }
    "#,
        "example",
    ));
    let question = scene.rank(Vertex::Node(NodeId::Block(0)));
    let gaps = vertical_gaps(&scene);

    assert!(gaps[question - 1] > MIN_VERTICAL_GAP);
    assert!(gaps[question] > MIN_VERTICAL_GAP);
    for (row, gap) in gaps.into_iter().enumerate() {
        if ![question - 1, question].contains(&row) {
            assert_eq!(gap, MIN_VERTICAL_GAP);
        }
    }
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

            |end| return end;
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
    assert!(
        scene
            .labels
            .iter()
            .all(|label| joined(&label.lines, "") != "proceed")
    );
    assert!(scene.labels.iter().any(|label| {
        matches!(label.kind, LabelKind::Wire) && joined(&label.lines, "") == "fallback"
    }));
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
fn math_branch_descriptions_clear_their_nodes_and_connections() {
    let scene = drawn((
        r#"
        #[kaalang]
        fn example(condition: bool) -> u8 {
            #[question("$x^2$ or $y^2$?")]
            #[no(r"$$\frac{\frac{1}{2}}{\frac{3}{4}}$$")]
            #[yes(r"$$\frac{\frac{1}{2}}{\frac{3}{4}}$$")]
            let (fallback, proceed) = |condition| condition;
            #[action("Use the fallback.")]
            let end = |fallback| 0;
            #[action("Proceed.")]
            let end = |proceed| 1;
            |end| return end;
        }
    "#,
        "example",
    ));
    let branch_labels = scene
        .labels
        .iter()
        .filter(|label| matches!(label.kind, LabelKind::Branch))
        .collect::<Vec<_>>();

    assert_eq!(branch_labels.len(), 2);
    for label in branch_labels {
        assert!(
            label
                .lines
                .iter()
                .flat_map(RichText::spans)
                .any(|span| span.formula.is_some())
        );
        // Display math is taller than one text line, so its label reserves more
        // room than the line count alone would give it.
        let rect = label_rect(label);
        assert!(rect.3 - rect.1 > 2 * LINE_HEIGHT, "{rect:?}");
        for node in &scene.nodes {
            assert!(
                !kaalang_compiler::geometry::overlaps(rect, Scene::bounds(node)),
                "{rect:?} meets the node at {},{}",
                node.x,
                node.y
            );
        }
        for connection in &scene.connections {
            assert!(
                !super::route::crosses(&connection.points, rect),
                "{rect:?} meets a connection"
            );
        }
    }
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
    assert_main_sequence(&scene, &[0, 1]);
    let end = scene.node(end_node(&scene));
    assert_eq!(end.x, scene.node(NodeId::Start).x);
    assert!(
        scene
            .topology
            .connections
            .contains(&kaalang_compiler::topology::Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(1))),
                destination: Destination::Node(end.id),
            })
    );
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

            |end| return end;
        }
    "#;
    let scene = drawn((source, "serial"));
    assert_main_sequence(&scene, &[0, 1, 2, 3, 4, 5, 6]);
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

/// The implicit visual end boundary, which `Flow::blocks` carries last.
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
fn start_separates_the_flow_name_and_typed_parameters() {
    let source = r"
        #[kaalang]
        fn example<'a, T>(r#type: &'a T, _: usize) -> &'a T
        where
            T: Copy,
        {
            |r#type| return r#type;
        }
    ";
    let scene = drawn((source, "example"));
    assert_eq!(
        scene.captions.label(NodeId::Start).as_ref(),
        "example<'a, T> where T: Copy,"
    );
    let parameters = scene.parameters.as_ref().expect("the flow has parameters");
    assert_eq!(parameters.parameters, ["r#type: &'a T", "_: usize"]);
    assert_eq!(
        parameters
            .lines
            .iter()
            .map(RichText::as_ref)
            .collect::<Vec<_>>(),
        parameters.parameters
    );
    assert!(parameters.x > scene.node(NodeId::Start).x);
}

#[test]
fn capsule_captions_fit_the_curved_outline() {
    for fields in [1, 12, 56] {
        let return_type = std::iter::repeat_n("[u8; 1]", fields)
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!(
            "#[kaalang] fn capsule(end: ({return_type})) -> ({return_type}) {{ |end| return end; }}"
        );
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
                    assert!(
                        inside,
                        "{id:?}: caption leaves the curved outline: {}",
                        line.as_ref()
                    );
                }
            }
        }
    }
}

#[test]
fn continuation_starts_below_the_completed_nested_cycle() {
    let scene = drawn(fixture!("loop/behavior", "nested_search"));
    let (inner, region) = scene
        .topology
        .loop_boundaries
        .iter()
        .zip(&scene.loop_regions)
        .find(|(_, region)| region.description == "Search the current row.")
        .expect("the inner cycle has a boundary");
    let outer = scene
        .topology
        .loop_boundaries
        .iter()
        .find(|boundary| (boundary.header + 1..boundary.end).contains(&inner.header))
        .expect("the inner cycle is enclosed");
    let outer = scene
        .topology
        .loops
        .iter()
        .find(|loop_| loop_.header == outer.header)
        .expect("the outer cycle repeats");
    let back = scene
        .connections
        .iter()
        .position(|edge| edge.source == Source::Junction(outer.tail))
        .expect("the outer cycle has a back edge");

    let mut crossed = scene.clone();
    let climb = crossed.connections[back]
        .points
        .windows(2)
        .position(|segment| segment[1].y < segment[0].y)
        .expect("the back edge climbs");
    for point in &mut crossed.connections[back].points[climb..=climb + 1] {
        point.x = i32::midpoint(region.left, region.right);
    }
    assert!(
        loop_block::verify(&crossed)
            .is_some_and(|reason| reason.contains("back edge runs too close")),
        "the boundary verifier must reject a foreign route through the cycle"
    );
}

#[test]
fn enclosing_back_edges_leave_a_lane_beside_nested_boundaries() {
    let mut close = drawn(fixture!("loop/behavior", "nested_exit_convergence"));
    let x = close.loop_regions[1].right + LANE - 1;
    let back = close
        .connections
        .iter_mut()
        .find(|edge| edge.source == Source::Junction(close.topology.loops[0].tail))
        .unwrap();
    for point in &mut back.points[1..3] {
        point.x = x;
    }
    assert_eq!(route::verify(&close), None);
    assert_eq!(
        loop_block::verify(&close).as_deref(),
        Some("cycle 0 back edge runs too close to cycle 1")
    );
}

#[test]
fn a_cycle_contains_the_wrapped_label_of_its_final_merge() {
    let source = include_str!("../../../kaalang/tests/gallery/binary_search/mod.rs");
    for repeats in [2, 8] {
        let source = source.replace("outcome", &"a_long_outcome_wire_name".repeat(repeats));
        let scene = drawn((&source, "binary_search"));
        let result = scene.topology.loop_boundaries[0].result.unwrap();
        let label = scene
            .labels
            .iter()
            .find(|label| label.owner == Vertex::from(result))
            .expect("the final merge retains its wire label");
        assert!(label.lines.len() > 1);
        assert!(label_rect(label).3 <= scene.loop_regions[0].bottom);
    }
}

#[test]
fn even_an_empty_cycle_draws_its_described_boundary() {
    let source = r#"
        #[kaalang]
        fn example() {
            #[cycle("Repeat forever.")]
            || {};
        }
    "#;
    let scene = drawn((source, "example"));
    let [region] = scene.loop_regions.as_slice() else {
        panic!("the expanded cycle has one boundary");
    };
    assert_eq!(region.description, "Repeat forever.");
    assert!(region.right > region.left);
    assert!(region.bottom > region.top);
    assert!(
        !scene
            .topology
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::End)
    );
}

#[test]
fn a_side_back_edge_clears_a_wrapped_branch_description() {
    let (source, flow) = fixture!("loop/behavior", "empty_trailing_loop");
    let description = "Restart the entire outer iteration after the inner cycle finishes.";
    let source = source.replace("#[no(\"NO\")]", &format!("#[no(\"{description}\")]"));
    let scene = drawn((&source, flow));
    let label = scene
        .labels
        .iter()
        .find(|label| joined(&label.lines, "") == description)
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

const IMPOSSIBLE_CYCLE: &str = r#"
    #[kaalang]
    fn impossible_cycle(mode: u8) -> u8 {
        #[cycle("Advance until the mode can leave.")]
        let result = |mut mode| {
            #[choice("Exit or advance?")]
            #[case("Advance from zero.")]
            #[case("Leave the cycle.")]
            #[case("Advance from another mode.")]
            let (first, leave, last) = |mode| match mode {
                0 => (),
                1 => (),
                _ => (),
            };

            #[action("Set the mode to one.")]
            |first, &mut mode| *mode = 1;

            |leave, mode| break mode;

            #[action("Set the mode to one.")]
            |last, &mut mode| *mode = 1;
        };

        |result| return result;
    }
"#;

#[test]
fn a_break_between_repeating_cases_is_rejected_before_layout() {
    let error = crate::render_source(IMPOSSIBLE_CYCLE, "impossible_cycle").unwrap_err();
    let crate::RenderError::InvalidFlow { message, .. } = &error else {
        panic!("an unrealizable topology is an invalid flow, not a rendering error: {error}")
    };
    // The realizability decision, not some earlier wire rule.
    assert!(
        message.contains("could not construct a diagram under RFC 0002"),
        "{message}"
    );
}

/// Pulling a back edge over its body is rejected by the final geometry check.
#[test]
fn a_back_edge_inside_its_body_is_caught_by_the_geometry_check() {
    let fixture = fixture!("loop/behavior", "trailing_inner_loop");
    let scene = drawn(fixture);
    for index in 0..scene.topology.loops.len() {
        let mut pulled = drawn(fixture);
        let tail = pulled.topology.loops[index].tail;
        let back = pulled
            .connections
            .iter()
            .position(|edge| edge.source == Source::Junction(tail))
            .expect("every repeating cycle draws its back edge");
        let inside = pulled.connections[back].points[0].x;
        for point in &mut pulled.connections[back].points[1..3] {
            point.x = inside;
        }
        let reason = route::verify(&pulled)
            .expect("a back edge pulled over its body or a nested back edge is rejected");
        assert!(
            reason.contains(&format!(
                "the iteration back edge of the cycle at block {} climbs inside",
                pulled.topology.loops[index].header + 1
            )),
            "{reason}"
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
        .expect("the cycle entry has an initial arrival");
    incoming
        .points
        .last_mut()
        .expect("the arrival has an endpoint")
        .y -= 1;

    // A gap creates no crossing. The routes must still meet at one point.
    assert!(correspondence(&scene).is_some());
    assert!(
        finish(scene).is_err(),
        "a disconnected junction must fail validation"
    );
}

#[test]
fn a_misplaced_end_is_caught_by_the_geometry_check() {
    let fixture = fixture!("loop/behavior", "end_below_nested_back_edges");
    let scene = drawn(fixture);
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
        end::verify(&raised).is_some(),
        "a node below end should be caught"
    );
    // Final geometry verification also checks end placement.
    assert_eq!(
        route::verify(&raised).as_deref(),
        Some("end must be below every other node and iteration back edge"),
        "the geometry gate should run the end rule"
    );

    // A back edge running down past it. Level with end's top edge is allowed —
    // the block still sits below the rail — so this has to go further.
    let mut trailing = drawn(fixture);
    let top = Scene::bounds(trailing.node(end)).1;
    let back = trailing
        .connections
        .iter()
        .position(|edge| trailing.is_back_edge(edge))
        .expect("the fixture has a back edge");
    assert!(
        {
            for point in &mut trailing.connections[back].points {
                point.y = top;
            }
            end::verify(&trailing).is_none()
        },
        "a back edge level with end's top edge is allowed"
    );
    for point in &mut trailing.connections[back].points {
        point.y = top + 1;
    }
    assert!(
        end::verify(&trailing).is_some(),
        "a back edge below end should be caught"
    );
}

fn named_node<'a>(scene: &'a Scene, description: &str) -> &'a Node {
    let id = scene
        .topology
        .nodes
        .iter()
        .find(|node| scene.captions.label(node.id).as_ref() == description)
        .expect("the fixture contains the described node")
        .id;
    scene.node(id)
}

/// A long label beside an iteration back edge remains clear after boundary placement.
#[test]
fn a_label_reaching_into_a_back_edge_gap_remains_clear() {
    let source = r#"
        #[kaalang]
        fn collect_steps(enabled: bool, limit: usize) -> Vec<String> {
            #[question("Collect the steps?")]
            let (run, skip) = |enabled| enabled;

            #[action("Start an empty log.")]
            let mut initial_log = |run| Vec::new();

            #[cycle("Collect the first steps.")]
            let log = |mut initial_log, &limit| {
                #[question("Are there more first steps?")]
                #[yes("YES")]
                #[no("NO")]
                let (iterate_1, leave_1) = |&initial_log, limit| initial_log.len() < *limit;

                |leave_1, initial_log| break initial_log;

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
                |&mut initial_log, wwwwwwwwwwwwwwwwwwwwwwww| initial_log.push(wwwwwwwwwwwwwwwwwwwwwwww);
            };

            #[cycle("Collect the second steps.")]
            let end = |mut log, &limit| {
                #[question("Are there more second steps?")]
                #[yes("YES")]
                #[no("NO")]
                let (iterate_2, leave_2) = |&log, limit| log.len() < *limit * 2;

                |leave_2, log| break log;

                #[question("Is the log length odd?")]
                let (odd, even) = |iterate_2, &log| log.len() % 2 == 1;

                #[action("Build an odd step.")]
                let wwwwwwwwwwwwwwwwwwwwwwww = |odd| String::from("b");

                #[action("Build an even step.")]
                let wwwwwwwwwwwwwwwwwwwwwwww = |even| String::from("c");

                #[action("Record the second step.")]
                |&mut log, wwwwwwwwwwwwwwwwwwwwwwww| log.push(wwwwwwwwwwwwwwwwwwwwwwww);
            };

            #[action("Return an empty log.")]
            let end = |skip| Vec::new();

            |end| return end;
        }
    "#;
    drawn((source, "collect_steps"));
}

/// Nested rails must preserve their horizontal order even with disjoint row
/// spans, where crossing checks cannot detect a misplaced contour.
#[test]
fn a_back_edge_inside_a_nested_back_edge_is_caught_by_the_geometry_check() {
    let source = r#"
        #[kaalang]
        fn nested_contours(mode: u8) -> u8 {
            #[cycle("Repeat the outer cycle.")]
            |mode| {
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

                #[cycle("Repeat the inner cycle.")]
                |third| {};
            };
        }
    "#;
    let scene = drawn((source, "nested_contours"));
    assert_eq!(scene.topology.loops.len(), 2, "an outer and an inner cycle");

    // Bring the inner back edge round to the outer one's side and past it. Both
    // stay clear of every body, and their rows still never meet, so only the
    // rule that holds an enclosing back edge outside a nested one objects.
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
        .expect("every repeating cycle draws its back edge");
    for point in &mut passed.connections[back].points[1..3] {
        point.x = beyond;
    }
    assert_eq!(
        route::verify(&passed).as_deref(),
        Some(
            format!(
                "the iteration back edge of the cycle at block {} climbs inside the back edge nested in its body at {beyond}",
                passed.topology.loops[0].header + 1
            )
            .as_str()
        ),
        "an enclosing back edge driven past a nested one should be caught"
    );
}

/// Where one cycle's back edge climbs.
fn rail(scene: &Scene, index: usize) -> i32 {
    scene
        .connections
        .iter()
        .find(|edge| edge.source == Source::Junction(scene.topology.loops[index].tail))
        .expect("every repeating cycle draws its back edge")
        .points[1]
        .x
}

/// Clearing a label moves the struck rail and its enclosing rails together;
/// moving only one would close the lane between them.
#[test]
fn a_label_past_a_nested_back_edge_moves_the_chain_outside_it() {
    let source = r#"
        #[kaalang]
        fn nested_exit_convergence(mut count: usize) -> usize {
            #[cycle("Count to completion.")]
            let result = |mut count| {
                #[cycle("Resolve the inner count.")]
                |&mut count| {
                    #[choice("Leave the inner loop?")]
                    #[case("Leave at zero.")]
                    #[case("Leave at one.")]
                    #[case("Count down.")]
                    let (zero, one, wwwwwwww) = |&count| match **count {
                        0 => (),
                        1 => (),
                        _ => (),
                    };

                    #[action("Finish at zero.")]
                    let done = |zero| {};
                    #[action("Finish at one.")]
                    let done = |one| {};
                    |done| break;

                    #[action("Count down.")]
                    |wwwwwwww, &mut count| **count -= 1;
                };

                #[question("Finish the outer loop?")]
                let (done, wwwwwwww) = |&count| *count == 0;

                |done, count| break count;

                #[action("Count down once more.")]
                |wwwwwwww, &mut count| *count -= 1;
            };

            |result| return result;
        }
    "#;
    let scene = drawn((source, "nested_exit_convergence"));
    let rails = (0..scene.topology.loops.len())
        .map(|index| rail(&scene, index))
        .collect::<Vec<_>>();
    assert_eq!(rails.len(), 2, "an outer and an inner cycle");
    assert!(
        rails[0].abs_diff(rails[1]) >= LANE.unsigned_abs(),
        "the two rails keep their lane apart: {rails:?}"
    );
}

#[test]
fn translating_the_drawing_preserves_its_back_edge_contours() {
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
    assert_eq!(correspondence(&scene), None);
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
        owner: Vertex::Node(NodeId::Start),
        kind: LabelKind::Wire,
        lines: vec![crate::text::RichText::literal("wwwwwwwwww")],
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

/// The probe `a_back_edge_may_stand_beyond_the_body_it_clears` checks in the
/// model: a cycle whose three cases repeat, repeat and leave.
const FAR_CONTOUR: (&str, &str) = (
    r#"
        #[kaalang]
        fn far_contour(mode: u8) -> u8 {
            #[cycle("Advance until the mode can leave.")]
            let result = |mut mode| {
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
                |case_2, mode| break mode;
            };

            |result| return result;
        }
    "#,
    "far_contour",
);

/// A far contour retains its recorded column; measuring only body boxes would
/// incorrectly pull the rail two columns inward.
#[test]
fn a_back_edge_beyond_its_body_is_drawn_where_the_arrangement_put_it() {
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

/// A sole tail arrival also keeps a separately recorded far contour.
/// The model test checks the same farther contour as a witness.
#[test]
fn a_sole_tail_arrival_keeps_its_recorded_far_contour() {
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
        "the witness must have one tail arrival"
    );
    let anchor = scene.node(NodeId::Start).x + scene.column_x(scene.arrangement.contours[0].column)
        - scene.column_x(scene.column(Vertex::Node(NodeId::Start)));
    let drawn_at = rail(&scene, 0);
    assert!(
        drawn_at >= anchor + LANE,
        "the back edge must stay outside its recorded column: {drawn_at} against {anchor}"
    );
}

/// Four cycles nested one inside the next. The model's
/// `four_nested_back_edges_climb_four_lanes_on_one_side` checks the same witness
/// in abstract columns; this one draws it.
const FOUR_LANES: (&str, &str) = (
    r#"
        #[kaalang]
        fn deep(mut step: usize) -> usize {
            #[cycle("Repeat the first cycle.")]
            let result = |mut step| {
                #[question("Leave the first?")]
                let (stay_0, leave_0) = |&step| *step > 0;
                |leave_0, step| break step;
                #[cycle("Repeat the second cycle.")]
                |stay_0, &mut step| {
                    #[question("Leave the second?")]
                    let (stay_1, leave_1) = |&step| **step > 1;
                    |leave_1| break;
                    #[cycle("Repeat the third cycle.")]
                    |stay_1, &mut step| {
                        #[question("Leave the third?")]
                        let (stay_2, leave_2) = |&step| **step > 2;
                        |leave_2| break;
                        #[cycle("Repeat the fourth cycle.")]
                        |stay_2, &mut step| {
                            #[question("Leave the fourth?")]
                            let (stay_3, leave_3) = |&step| **step > 3;
                            |leave_3| break;
                            #[action("Advance at the deepest level.")]
                            |stay_3, &mut step| **step += 1;
                        };
                    };
                };
            };

            |result| return result;
        }
    "#,
    "deep",
);

/// Four nested rails reach lane 3, beyond ordinary fixture coverage. Each stays
/// a lane outside the one it encloses, with the outermost four lanes from the body.
#[test]
fn four_nested_back_edges_are_drawn_in_four_lanes() {
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

    let crowded = match side {
        Side::Left => scene.arrangement.contours[0].column - 1,
        Side::Right => scene.arrangement.contours[0].column,
    };
    let mut without_contours = scene.clone();
    without_contours.reach.clear();
    let ordinary = scene
        .arrangement
        .column
        .values()
        .copied()
        .find(|&left| {
            without_contours.column_gap(left) == COLUMN_WIDTH + scene.slack
                && scene.column_gap(left) == without_contours.column_gap(left)
        })
        .expect("the probe has an ordinary gap between content columns");
    assert!(
        scene.column_gap(crowded) > without_contours.column_gap(crowded),
        "the deep contour should widen its own gap"
    );
    assert_eq!(
        scene.column_gap(ordinary),
        without_contours.column_gap(ordinary),
        "the deep contour should not widen another gap"
    );
}

/// Accepted generated cycles render except for one pinned boundary crossing.
/// Extends coverage beyond authored fixtures.
#[test]
fn every_generated_shape_the_model_accepts_also_renders() {
    let sources = kaalang_testing::shapes::loop_shapes()
        .into_iter()
        .chain(kaalang_testing::shapes::question_shapes())
        .map(|source| format!("#[kaalang]\n{source}"))
        .collect::<Vec<_>>();
    assert_eq!(sources.len(), 2835);

    let (mut drawn, mut refused, mut undrawable) = (0, 0, 0);
    for source in &sources {
        let file = crate::parse_file(source).expect("the probe is valid Rust");
        let function = crate::select_flow(&file.items, "probe").expect("the probe declares it");
        let mut model = match kaalang_compiler::build(&function) {
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
        kaalang_render::compact_arrangement(&mut model);
        let start = start_text(source, &function.sig);
        let parameters = parameter_text(source, &function.sig);
        let captions = captions_of(&model, &start, &return_text(source, &function.sig.output));
        if let Err(reason) = layout(&model, &captions, &parameters) {
            // One shape leaves the model with an arrangement whose route
            // passes through a nested cycle's columns. The rectangle is
            // checked on compaction candidates and the searches stay unaware
            // of it, so this one reaches the renderer and the renderer is
            // right to refuse it. Pinned rather than tolerated: it is the
            // whole gap, and it must not widen.
            assert!(
                reason.contains("is crossed by external route"),
                "{source}\nan accepted flow did not render: {reason}"
            );
            undrawable += 1;
        }
    }
    assert_eq!((drawn, refused, undrawable), (1091, 1744, 1));
}

/// Moving the climb alone can keep the route connected while violating the
/// contour chosen by the arrangement.
#[test]
fn a_back_edge_keeps_its_recorded_contour() {
    let mut scene = drawn(FAR_CONTOUR);
    let back = scene
        .connections
        .iter()
        .position(|edge| scene.is_back_edge(edge))
        .unwrap();
    let climb = scene.connections[back]
        .points
        .windows(2)
        .position(|pair| pair[1].y < pair[0].y)
        .unwrap();
    let anchor = route::contour_anchor(&scene, 0);
    scene.connections[back].points[climb].x = anchor;
    scene.connections[back].points[climb + 1].x = anchor;
    assert!(
        correspondence(&scene)
            .unwrap()
            .contains("moved inside its recorded contour")
    );
}

#[test]
fn a_back_edge_clears_its_drawn_entry_and_tail() {
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
        // This right back edge now stands left of its own endpoint. No other
        // body vertex moved, so the body-extents check alone cannot see it.
        points[endpoint].x = points[1].x + 1;
        assert!(
            route::verify(&moved)
                .unwrap()
                .contains("climbs inside its body")
        );
    }
}
