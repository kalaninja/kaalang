use kaalang_model::SemanticModel;
use kaalang_model::topology::{Connection, Destination, ExitId, NodeId, Source, Vertex};

use super::{Captions, derive};

fn read(source: &str) -> (SemanticModel, Captions) {
    let function = syn::parse_str(source).expect("the flow parses");
    let model = kaalang_model::build(&function).expect("the flow is valid");
    let captions = derive(&model, "example", "u8");
    (model, captions)
}

/// One flow of an authored fixture file. The start and return type only caption
/// start and end, which no test here reads.
fn fixture(source: &str, flow: &str) -> (SemanticModel, Captions) {
    let file = crate::parse_file(source).expect("the fixture parses");
    let function = crate::select_flow(&file.items, flow).expect("the fixture declares its flow");
    let model = kaalang_model::build(&function).expect("the fixture is valid");
    let captions = derive(&model, "example", "u8");
    (model, captions)
}

#[test]
fn labels_belong_to_exits_and_nodes_including_unused_names() {
    let (_, captions) = read(
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

            |end| return end;
        }
    "#,
    );
    assert_eq!(
        captions.handover(ExitId::of(NodeId::Start)),
        ["type", "_spare"]
    );
    assert_eq!(captions.capture(NodeId::Block(0)), ["type"]);
    assert_eq!(
        captions.handover(ExitId::of(NodeId::Block(1))),
        ["width", "depth", "_unused"]
    );
    assert_eq!(captions.capture(NodeId::Block(2)), ["&width", "depth"]);
    let distributor = ExitId::of(NodeId::Block(0));
    assert!(captions.handover(distributor).is_empty());
    for (branch, name) in ["near", "far"].into_iter().enumerate() {
        let case = NodeId::Case { choice: 0, branch };
        assert!(captions.capture_label(case).is_empty());
        assert_eq!(captions.handover(ExitId::of(case)), [name]);
        assert_eq!(
            captions.label(case),
            if branch == 0 { "Near." } else { "Far." }
        );
        // The distributor is the fan-out this projection actually produces:
        // both displayed lists are empty and therefore equal, and RFC 0002 §6
        // still keeps the two ends apart because the exit has two connections.
        assert!(!captions.shares_label(&Connection {
            source: Source::Exit(distributor),
            destination: Destination::Node(case),
        }));
    }
    assert_eq!(captions.label(NodeId::Block(0)), "Pick a case.");
    assert_eq!(captions.label(NodeId::Start), "example");
    assert_eq!(captions.label(NodeId::Block(5)), "u8");
    assert_eq!(captions.capture(NodeId::Block(5)), ["end"]);
}

#[test]
fn end_names_the_transferred_value_instead_of_all_return_captures() {
    for (source, expected) in [
        (
            r#"
            fn example(value: u8) -> u8 {
                #[action("Keep a guard alive through return.")]
                let guard = || ();
                |value, guard| return value;
            }
            "#,
            &["value"][..],
        ),
        (
            r"
            fn example(left: u8, right: u8) -> (u8, u8) {
                |left, right| return (left, right);
            }
            ",
            &["left", "right"][..],
        ),
        (
            r"
            fn example(value: u8) -> (u8,) {
                |value| return (value,);
            }
            ",
            &["value"][..],
        ),
        (
            r"
            fn example(value: (u8, u8)) -> (u8, u8) {
                |value| return value;
            }
            ",
            &["value"][..],
        ),
        (
            r"
            fn example(value: u8) -> u8 {
                |value| return (value);
            }
            ",
            &["value"][..],
        ),
        (
            r"
            fn example() {
                return ();
            }
            ",
            &["()"][..],
        ),
    ] {
        let (model, captions) = read(source);
        let end = model
            .topology
            .nodes
            .iter()
            .find(|node| node.kind == kaalang_model::topology::NodeKind::End)
            .expect("the flow returns")
            .id;
        assert_eq!(captions.capture(end), expected);
        assert!(model.topology.junctions.is_empty());
        assert_eq!(model.topology.incoming(Vertex::Node(end)).count(), 1);
    }
}

#[test]
fn a_tuple_return_shares_the_producers_ordered_wire_label() {
    let (model, captions) = read(
        r"
        fn example(left: u8, right: u8) -> (u8, u8) {
            |left, right| return (left, right);
        }
        ",
    );
    let [connection] = model.topology.connections.as_slice() else {
        panic!("the zero-computation flow connects start directly to end");
    };
    assert!(captions.shares_label(connection));
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
        let (_, captions) = read(&format!(
            r#"
            fn example() -> u8 {{
                #[action("Prepare two values.")]
                let ({outputs}) = || {{ (1, 2) }};
                #[action("Use both values.")]
                let end = |{capture}| {{ 3 }};

                |end| return end;
            }}
        "#
        ));
        assert_eq!(captions.capture(NodeId::Block(0)), Vec::<String>::new());
        assert_eq!(
            captions.handover(ExitId::of(NodeId::Block(0))),
            outputs.split(", ").collect::<Vec<_>>()
        );
        assert_eq!(captions.capture_label(NodeId::Block(0)), ["()"]);
        assert_eq!(
            captions.capture(NodeId::Block(1)),
            capture.split(", ").collect::<Vec<_>>()
        );
        assert_eq!(
            captions.shares_label(&Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(0))),
                destination: Destination::Node(NodeId::Block(1)),
            }),
            shared
        );
    }
}

/// A branch-local block written above a merge invents no hand-over and no
/// capture.
#[test]
fn a_branch_local_block_before_a_merge_invents_no_label() {
    let (_, captions) = fixture(
        include_str!("../../../kaalang/tests/wire/behavior/local_work_before_a_wire_merge.rs"),
        "local_work_before_a_wire_merge",
    );
    let note = NodeId::Block(3);
    assert_eq!(captions.junction_wires(0), ["selected", "noted"]);
    assert_eq!(captions.capture(note), ["local_note", "&order"]);
    assert_eq!(captions.handover(ExitId::of(note)), ["noted"]);
    assert_eq!(captions.capture(NodeId::Block(4)), ["selected", "&order"]);
}

#[test]
fn mutable_alternatives_keep_the_same_label_across_block_kinds_and_the_merge() {
    let source =
        include_str!("../../../kaalang/tests/capture/behavior/mutate_merged_branch_outputs.rs");
    let (_, captions) = fixture(source, "mutate_merged_branch_outputs");
    for exit in [
        ExitId {
            node: NodeId::Block(0),
            branch: Some(0),
        },
        ExitId::of(NodeId::Case {
            choice: 1,
            branch: 0,
        }),
        ExitId::of(NodeId::Block(2)),
    ] {
        assert_eq!(captions.handover(exit), ["mut selected"]);
    }
    assert_eq!(captions.junction_wires(0), ["mut selected"]);
}

#[test]
fn a_question_branch_description_replaces_its_output_label() {
    let (_, captions) = read(
        r#"
        fn example(condition: bool) -> u8 {
            #[question("Ready?")]
            #[yes("Go ahead.")]
            #[no]
            let (yes, no) = |condition| { condition };
            #[action("Finish now.")]
            let end = |yes| { 1 };
            #[action("Finish later.")]
            let end = |no| { 2 };

            |end| return end;
        }
    "#,
    );
    let yes = ExitId {
        node: NodeId::Block(0),
        branch: Some(0),
    };
    let no = ExitId {
        node: NodeId::Block(0),
        branch: Some(1),
    };
    assert_eq!(captions.branch_description(yes), Some("Go ahead."));
    assert_eq!(captions.branch_description(no), None);
    // The hand-over is still recorded; the description replaces it at drawing
    // time rather than removing it.
    assert_eq!(captions.handover(yes), ["yes"]);
    assert_eq!(captions.handover(no), ["no"]);
}

#[test]
fn a_call_without_a_description_is_labeled_with_the_path_it_calls() {
    let (_, captions) = read(
        "fn example(value: u32) -> u32 {
            #[call]
            let end = |value| math::
                // The author may break a path across lines.
                twice(value);

            |end| return end;
        }",
    );
    assert_eq!(captions.label(NodeId::Block(0)), "math::twice");
}
