use kaalang_model::SemanticModel;
use kaalang_model::topology::{Connection, Destination, ExitId, NodeId, Source};

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
    let model = kaalang_model::build(function).expect("the fixture is valid");
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
    assert_eq!(captions.label(NodeId::Block(4)), "u8");
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
