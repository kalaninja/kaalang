use super::*;

fn drawn(source: &str) -> Topology {
    let function = syn::parse_str(source).unwrap();
    project(&kaalang_model::build(&function).unwrap(), "example", "u8")
}

/// One flow of an authored fixture file. The start and return type only
/// label start and end, which no test here reads.
fn fixture(source: &str, flow: &str) -> Topology {
    let file = crate::parse_file(source).unwrap();
    let function = crate::select_flow(&file.items, flow).unwrap();
    project(&kaalang_model::build(function).unwrap(), "example", "u8")
}

/// Whether any chain of connections leads from one vertex to another.
fn reaches(topology: &Topology, from: Vertex, to: Vertex) -> bool {
    let mut frontier = vec![from];
    let mut seen = BTreeSet::from([from]);
    while let Some(vertex) = frontier.pop() {
        if vertex == to {
            return true;
        }
        for connection in topology.outgoing(vertex) {
            let next = connection.destination;
            if seen.insert(next) {
                frontier.push(next);
            }
        }
    }
    false
}

#[test]
fn an_empty_unconditional_loop_uses_only_its_entry_and_tail() {
    let topology = drawn(
        r"
        fn example() -> usize {
            loop {}
        }
    ",
    );
    let loop_ = topology.loops[0];
    assert_eq!(topology.nodes.len(), 1);
    assert_eq!(topology.nodes[0].id, NodeId::Start);
    assert_eq!(topology.junctions.len(), 2);
    assert_eq!(
        topology.connections,
        [
            Connection {
                source: Source::Exit(ExitId::of(NodeId::Start)),
                destination: Destination::Junction(loop_.entry),
            },
            Connection {
                source: Source::Junction(loop_.entry),
                destination: Destination::Junction(loop_.tail),
            },
        ]
    );
    assert_eq!(
        topology.back_edges,
        [Connection {
            source: Source::Junction(loop_.tail),
            destination: Destination::Junction(loop_.entry),
        }]
    );
}

#[test]
fn a_terminal_loop_finishes_before_its_enclosing_end_merge() {
    let topology = drawn(
        r#"
        fn example(flag: bool) -> usize {
            #[question("Flag?")]
            while (|flag| flag) {
                loop {
                    #[action("Return one.")]
                    let end = || 1;
                }
            }
            #[action("Return two.")]
            let end = || 2;
        }
        "#,
    );
    assert!(topology.loops.is_empty());
    assert_eq!(topology.junctions.len(), 1);
    assert_eq!(topology.incoming(Vertex::Junction(0)).count(), 2);
    for block in [2, 3] {
        assert!(topology.connections.contains(&Connection {
            source: Source::Exit(ExitId::of(NodeId::Block(block))),
            destination: Destination::Junction(0),
        }));
    }
    assert!(topology.connections.contains(&Connection {
        source: Source::Junction(0),
        destination: Destination::Node(NodeId::Block(4)),
    }));
}

#[test]
fn a_final_nested_while_closes_before_the_unconditional_loop_repeats() {
    let topology = drawn(
        r#"
        fn example(flag: bool) -> usize {
            loop {
                #[question("Flag?")]
                while (|&flag| *flag) {}
            }
        }
        "#,
    );
    let [outer, inner] = topology.loops[..] else {
        panic!("both loops repeat");
    };
    assert!(topology.connections.contains(&Connection {
        source: Source::Exit(ExitId {
            node: NodeId::Block(inner.header),
            branch: Some(0),
        }),
        destination: Destination::Junction(inner.tail),
    }));
    assert!(topology.connections.contains(&Connection {
        source: Source::Exit(ExitId {
            node: NodeId::Block(inner.header),
            branch: Some(1),
        }),
        destination: Destination::Junction(outer.tail),
    }));
    assert!(topology.order.contains(&Connection {
        source: Source::Junction(inner.tail),
        destination: Destination::Junction(outer.tail),
    }));
}

#[test]
fn reduction_preserves_a_direct_branch_beside_a_longer_branch() {
    let topology = drawn(
        r#"
        fn example(condition: bool) -> u8 {
            #[question("Use the value directly?")]
            let (value, needs_work) = |condition| { condition };
            #[action("Prepare the other value.")]
            let value = |needs_work| { () };
            #[action("Use the merged value.")]
            let end = |value| { 1 };
        }
    "#,
    );
    for (branch, destination) in [
        (0, Destination::Junction(0)),
        (1, Destination::Node(NodeId::Block(1))),
    ] {
        let exit = ExitId {
            node: NodeId::Block(0),
            branch: Some(branch),
        };
        assert_eq!(
            topology.leaving(exit).copied().collect::<Vec<_>>(),
            [Connection {
                source: Source::Exit(exit),
                destination
            }]
        );
    }
    assert_eq!(topology.incoming(Vertex::Junction(0)).count(), 2);
    assert_eq!(topology.incoming(Vertex::Node(NodeId::Block(2))).count(), 1);
    assert!(topology.connections.contains(&Connection {
        source: Source::Junction(0),
        destination: Destination::Node(NodeId::Block(2)),
    }));
}

#[test]
fn labels_belong_to_exits_and_nodes_including_unused_names() {
    let topology = drawn(
        r#"
        fn example(r#type: u8, _spare: u8, _: u8) -> u8 {
            #[choice("Pick a case.")]
            #[case("Near.")]
            #[case("Far.")]
            let (near, far) = |r#type| { match r#type { 0 => 1, _ => 2 } };
            #[action("Split the near value.")]
            let (width, depth, _unused) = |near| { (near, near, ()) };
            #[action("Use both dimensions.")]
            let end = |&width, depth| { *width + depth };
            #[action("Use the far value.")]
            let end = |far| { far };
        }
    "#,
    );
    assert_eq!(
        topology.handover(ExitId::of(NodeId::Start)),
        ["type", "_spare"]
    );
    assert_eq!(topology.capture(NodeId::Block(0)), ["type"]);
    assert_eq!(
        topology.handover(ExitId::of(NodeId::Block(1))),
        ["width", "depth", "_unused"]
    );
    assert_eq!(topology.capture(NodeId::Block(2)), ["&width", "depth"]);
    let distributor = ExitId::of(NodeId::Block(0));
    assert!(topology.handover(distributor).is_empty());
    assert_eq!(topology.leaving(distributor).count(), 2);
    for (branch, name) in ["near", "far"].into_iter().enumerate() {
        let case = NodeId::Case { choice: 0, branch };
        assert!(topology.capture_label(case).is_empty());
        assert_eq!(topology.handover(ExitId::of(case)), [name]);
        let connection = Connection {
            source: Source::Exit(distributor),
            destination: Destination::Node(case),
        };
        assert_eq!(
            topology
                .incoming(Vertex::Node(case))
                .copied()
                .collect::<Vec<_>>(),
            [connection]
        );
        // The distributor is the fan-out this projection actually produces:
        // both displayed lists are empty and therefore equal, and RFC 0002 §6
        // still keeps the two ends apart because the exit has two connections.
        assert!(!topology.shares_label(&connection));
    }
    assert_eq!(
        topology
            .exits
            .iter()
            .map(|exit| exit.id)
            .collect::<BTreeSet<_>>()
            .len(),
        topology.exits.len()
    );
    assert_eq!(
        topology
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>()
            .len(),
        topology.nodes.len()
    );
    assert!(
        !topology
            .exits
            .iter()
            .any(|exit| exit.id.node == NodeId::Block(4))
    );
}

#[test]
fn adjacent_labels_share_only_identical_ordered_captures() {
    for (outputs, capture, shared) in [
        ("first, second", "first, second", true),
        ("first, second", "second, first", false),
        ("first, second", "&first, second", false),
        ("first, second", "mut first, second", false),
        ("mut first, second", "&mut first, second", false),
        ("mut first, second", "mut first, second", true),
        ("mut first, second", "first, second", false),
    ] {
        let topology = drawn(&format!(
            r#"
            fn example() -> u8 {{
                #[action("Prepare two values.")]
                let ({outputs}) = || {{ (1, 2) }};
                #[action("Use both values.")]
                let end = |{capture}| {{ 3 }};
            }}
        "#
        ));
        assert_eq!(topology.capture(NodeId::Block(0)), Vec::<String>::new());
        assert_eq!(
            topology.handover(ExitId::of(NodeId::Block(0))),
            outputs.split(", ").collect::<Vec<_>>()
        );
        assert_eq!(topology.capture_label(NodeId::Block(0)), ["()"]);
        assert_eq!(
            topology.capture(NodeId::Block(1)),
            capture.split(", ").collect::<Vec<_>>()
        );
        assert_eq!(
            topology.connections.len(),
            3,
            "the serial chain has no shortcut"
        );
        assert_eq!(
            topology.shares_label(&Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(0))),
                destination: Destination::Node(NodeId::Block(1)),
            }),
            shared
        );
    }
}

#[test]
fn a_merge_written_above_a_question_reaches_it_first() {
    let source =
        include_str!("../../../kaalang/tests/wire/behavior/a_branch_captures_a_merged_value.rs");
    let topology = fixture(source, "a_branch_captures_a_merged_value");
    let question = NodeId::Block(3);
    let connection = Connection {
        source: Source::Junction(0),
        destination: Destination::Node(question),
    };
    assert_eq!(topology.junctions[0].wires, ["counted", "seen"]);
    assert_eq!(
        topology
            .incoming(Vertex::Node(question))
            .copied()
            .collect::<Vec<_>>(),
        [connection]
    );
    assert_eq!(
        topology
            .outgoing(Vertex::Junction(0))
            .copied()
            .collect::<Vec<_>>(),
        [connection]
    );
    assert_eq!(topology.capture(question), ["verbose"]);
    assert!(crate::render_source(source, "a_branch_captures_a_merged_value").is_ok());
}

/// RFC 0002 §7 gives a question branch or a case exit at most one connection,
/// however many blocks capture the merged wire: the alternatives meet at the
/// junction and the common segment leaves it.
#[test]
fn a_merged_wire_leaves_each_branch_exit_once() {
    let two_consumers = drawn(
        r#"
        fn example(condition: bool) -> u8 {
            #[question("Choose a value?")]
            let (yes, no) = |condition| { condition };
            #[action("Build the yes value.")]
            let value = |yes| { 1 };
            #[action("Build the no value.")]
            let value = |no| { 2 };
            #[action("Double the merged value.")]
            let doubled = |&value| { *value * 2 };
            #[action("Add both.")]
            let end = |value, doubled| { value + doubled };
        }
    "#,
    );
    let only_end = fixture(
        include_str!("../../../kaalang/tests/question/behavior/run_question.rs"),
        "run_question",
    );

    // The consumers of the merged wire; the last block of each flow is end.
    for (topology, consumers) in [
        (&two_consumers, &[NodeId::Block(3), NodeId::Block(4)][..]),
        (&only_end, &[NodeId::Block(3)][..]),
    ] {
        for exit in &topology.exits {
            assert!(
                topology.leaving(exit.id).count() <= 1,
                "{:?} leaves more than once",
                exit.id
            );
        }
        assert_eq!(topology.junctions.len(), 1);
        assert_eq!(topology.incoming(Vertex::Junction(0)).count(), 2);
        // One common segment leaves the junction however many blocks capture
        // the wire; later consumers are reached through the first.
        assert_eq!(topology.outgoing(Vertex::Junction(0)).count(), 1);
        for &consumer in consumers {
            assert!(
                reaches(topology, Vertex::Junction(0), Vertex::Node(consumer)),
                "{consumer:?} is not reached from the junction"
            );
        }
    }
}

/// A branch-local block written above a merge invents no hand-over and no
/// capture. The junction lies on the path from both producers and from the
/// branch-local note block.
#[test]
fn a_branch_local_block_before_a_merge_invents_no_label() {
    let topology = fixture(
        include_str!("../../../kaalang/tests/wire/behavior/local_work_before_a_wire_merge.rs"),
        "local_work_before_a_wire_merge",
    );
    let note = NodeId::Block(3);
    // `selected` and `noted` arrive from the same two exits, so they share one
    // junction.
    assert_eq!(topology.junctions.len(), 1);
    assert_eq!(topology.junctions[0].wires, ["selected", "noted"]);

    for source in [NodeId::Block(1), NodeId::Block(2), note] {
        assert!(
            reaches(&topology, Vertex::Node(source), Vertex::Junction(0)),
            "{source:?} does not reach the junction"
        );
    }

    // The note block keeps its authored labels: reaching the junction adds
    // neither a hand-over of the merged wire nor a capture of it.
    assert_eq!(topology.capture(note), ["local_note", "&order"]);
    assert_eq!(topology.handover(ExitId::of(note)), ["noted"]);
    assert_eq!(topology.capture(NodeId::Block(4)), ["selected", "&order"]);
}

#[test]
fn mutable_alternatives_keep_the_same_label_across_block_kinds_and_the_merge() {
    let source =
        include_str!("../../../kaalang/tests/capture/behavior/mutate_merged_branch_outputs.rs");
    let topology = fixture(source, "mutate_merged_branch_outputs");
    for exit in [
        super::question::exit(0, 0),
        super::choice::exit(1, 0),
        super::action::exit(2),
    ] {
        assert_eq!(topology.handover(exit), ["mut selected"]);
    }
    assert_eq!(topology.junctions[0].wires, ["mut selected"]);
}
