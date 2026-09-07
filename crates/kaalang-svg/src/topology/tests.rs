use super::*;

fn drawn(source: &str) -> Topology {
    let function = syn::parse_str(source).unwrap();
    project(
        &kaalang_model::build(&function).unwrap(),
        "example",
        "-> u8",
    )
}

#[test]
fn reduction_preserves_a_direct_branch_beside_a_longer_branch() {
    let topology = drawn(
        r#"
        fn example(condition: bool) -> u8 {
            #[question("Use the value directly?")]
            |condition| -> (value, needs_work) { condition };
            #[action("Prepare the other value.")]
            |needs_work| -> value { () };
            #[action("Use the merged value.")]
            |value| -> result { 1 };
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
    assert_eq!(topology.arriving(NodeId::Block(2)).count(), 1);
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
            |r#type| -> (near, far) { match r#type { 0 => 1, _ => 2 } };
            #[action("Split the near value.")]
            |near| -> (width, depth, _unused) { (near, near, ()) };
            #[action("Use both dimensions.")]
            |&width, depth| -> result { *width + depth };
            #[action("Use the far value.")]
            |far| -> result { far };
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
        assert_eq!(
            topology.arriving(case).copied().collect::<Vec<_>>(),
            [Connection {
                source: Source::Exit(distributor),
                destination: Destination::Node(case),
            }]
        );
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
    for (capture, shared) in [
        ("first, second", true),
        ("second, first", false),
        ("&first, second", false),
    ] {
        let topology = drawn(&format!(
            r#"
            fn example() -> u8 {{
                #[action("Prepare two values.")]
                || -> (first, second) {{ (1, 2) }};
                #[action("Use both values.")]
                |{capture}| -> result {{ 3 }};
            }}
        "#
        ));
        assert_eq!(topology.capture(NodeId::Block(0)), Vec::<String>::new());
        assert_eq!(topology.capture_label(NodeId::Block(0)), ["()"]);
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
fn a_merge_precedes_the_question_that_selects_its_consumers() {
    let source =
        include_str!("../../../kaalang/tests/wire/behavior/a_branch_captures_a_merged_value.rs");
    let file = syn::parse_file(source).unwrap();
    let function = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "a_branch_captures_a_merged_value" => {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let model = kaalang_model::build(function).unwrap();
    let topology = project(&model, "example", "-> u8");
    let question = NodeId::Block(3);
    let connection = Connection {
        source: Source::Junction(0),
        destination: Destination::Node(question),
    };
    assert_eq!(topology.junctions[0].wires, ["counted", "seen"]);
    assert_eq!(
        topology.arriving(question).copied().collect::<Vec<_>>(),
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
