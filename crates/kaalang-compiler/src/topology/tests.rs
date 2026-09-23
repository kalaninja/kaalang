use std::collections::BTreeSet;

use super::*;
use crate::model::SemanticModel;
use crate::tests::model;

fn drawn(source: &str) -> Topology {
    model(source).topology
}

fn collapsed(source: &str) -> Topology {
    let function = syn::parse_str(source).expect("the flow parses");
    crate::build_with_options(&function, true)
        .expect("the flow is valid")
        .topology
}

/// One flow of an authored fixture file.
fn fixture(source: &str, flow: &str) -> SemanticModel {
    crate::build(&crate::tests::fixture(source, flow)).expect("the fixture is valid")
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

/// The wires that meet at one junction, as a consumer would display them.
fn merged(model: &SemanticModel, junction: usize) -> Vec<String> {
    model.topology.junctions[junction]
        .merges
        .iter()
        .map(|&merge| {
            model
                .analysis
                .flow
                .wire_name(&model.analysis.merges[merge].wire)
        })
        .collect()
}

#[test]
fn an_empty_unconditional_cycle_uses_only_its_entry_and_tail() {
    let topology = drawn(
        r#"
        fn example() -> usize {
            #[cycle("Repeat forever.")]
            || {};
        }
    "#,
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
fn a_fully_diverging_collapsed_cycle_has_no_normal_exit() {
    let topology = collapsed(
        r#"
        fn forever() -> ! {
            #[cycle("Never completes.")]
            let _result = || {};
        }
        "#,
    );
    let cycle = NodeId::Block(0);
    assert_eq!(topology.node(cycle).kind, NodeKind::Loop);
    assert!(!topology.exits.iter().any(|exit| exit.id.node == cycle));
    assert!(topology.outgoing(Vertex::Node(cycle)).next().is_none());
}

#[test]
fn a_collapsed_cycle_omits_its_internal_choice_connections() {
    let function = crate::tests::fixture(
        include_str!("../../../kaalang/tests/loop/behavior/diverging_middle_branch.rs"),
        "diverging_middle_branch",
    );
    let topology = crate::build_with_options(&function, true)
        .expect("the collapsed cycle hides its internal choice")
        .topology;

    assert!(
        topology
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Select | NodeKind::Case))
    );
}

#[test]
fn capture_free_transfers_redirect_without_structural_junctions() {
    let topology = drawn(
        r#"
        fn example() {
            #[cycle("Complete immediately.")]
            let () = || {
                break;
            };
            return;
        }
        "#,
    );
    let boundary = topology.loop_boundaries[0];
    let result = boundary.result_junction().expect("the cycle completes");
    let end = topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .expect("the flow returns")
        .id;

    assert_eq!(topology.junctions.len(), 2);
    assert!(topology.junctions.iter().all(|junction| !junction.is_break));
    assert!(topology.connections.contains(&Connection {
        source: Source::Junction(boundary.entry_junction()),
        destination: Destination::Junction(result),
    }));
    assert!(topology.connections.contains(&Connection {
        source: Source::Junction(result),
        destination: Destination::Node(end),
    }));
}

#[test]
fn nested_completing_cycles_reach_the_root_return() {
    let topology = drawn(
        r#"
        fn example(flag: bool) -> usize {
            #[cycle("Choose the result.")]
            let result = |flag| {
                #[question("Flag?")]
                let (iterate_1, leave_1) = |flag| flag;
                #[action("Return two.")]
                let selected = |leave_1| 2;
                #[cycle("Produce one.")]
                let selected = |iterate_1| {
                    #[action("Return one.")]
                    let one = || 1;
                    |one| break one;
                };
                |selected| break selected;
            };
            |result| return result;
        }
        "#,
    );
    assert!(topology.loops.is_empty());
    let end = topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .expect("the returning flow has an end boundary")
        .id;
    for break_ in topology
        .junctions
        .iter()
        .enumerate()
        .filter_map(|(index, junction)| junction.is_break.then_some(index))
    {
        assert!(reaches(
            &topology,
            Vertex::Junction(break_),
            Vertex::Node(end)
        ));
    }
}

#[test]
fn an_inner_break_reaches_the_outer_iteration_tail() {
    let topology = drawn(
        r#"
        fn example(flag: bool) -> usize {
            #[cycle("Repeat the outer cycle.")]
            |flag| {
                #[cycle("Leave or repeat the inner cycle.")]
                |flag| {
                    #[question("Flag?")]
                    let (_iterate_2, leave_2) = |flag| flag;
                    |leave_2| break;
                };
            };
        }
        "#,
    );
    let [outer, inner] = topology.loops[..] else {
        panic!("both loops repeat");
    };
    assert!(topology.connections.contains(&Connection {
        source: Source::Exit(ExitId {
            node: NodeId::Block(inner.header + 1),
            branch: Some(0),
        }),
        destination: Destination::Junction(inner.tail),
    }));
    let leave = ExitId {
        node: NodeId::Block(inner.header + 1),
        branch: Some(1),
    };
    assert!(topology.leaving(leave).any(|connection| reaches(
        &topology,
        connection.destination,
        Vertex::Junction(outer.tail)
    )));
}

#[test]
fn a_merged_break_reaches_the_enclosing_iteration_tail() {
    let model = model(
        r#"
        fn example(first: bool, second: bool) {
            #[cycle("Repeat the outer cycle.")]
            |first, second| {
                #[cycle("Leave or repeat the inner cycle.")]
                |first, second| {
                    #[question("Leave immediately?")]
                    let (leave, check) = |first| first;

                    #[question("Leave after checking?")]
                    let (leave, _again) = |check, second| second;

                    |leave| break;
                };
            };
        }
        "#,
    );
    let topology = &model.topology;
    let merge = (0..topology.junctions.len())
        .find(|&junction| merged(&model, junction) == ["leave"])
        .expect("the question outputs merge before the break");
    assert_eq!(
        topology.loop_boundaries[1].result,
        Some(Source::Junction(merge))
    );
    assert!(topology.junctions[merge].is_break);
    assert!(topology.junctions[merge].is_loop_result);
    assert!(reaches(
        topology,
        Vertex::Junction(merge),
        Vertex::Junction(topology.loops[0].tail)
    ));
}

#[test]
fn work_after_a_merge_keeps_the_cycle_result_separate() {
    let model = model(
        r#"
        fn example(flag: bool) {
            #[cycle("Choose, then finish.")]
            |flag| {
                #[question("Which route?")]
                let (done, other) = |flag| flag;
                #[action("Finish the other route.")]
                let done = |other| {};
                #[action("Finish after the merge.")]
                |done| {};
                |done| break;
            };
            return;
        }
        "#,
    );
    let topology = &model.topology;
    let merge = (0..topology.junctions.len())
        .find(|&junction| merged(&model, junction) == ["done"])
        .unwrap();
    let result = topology.loop_boundaries[0].result.unwrap();
    assert_eq!(result, Source::Exit(ExitId::of(NodeId::Block(3))));
    assert!(reaches(topology, Vertex::Junction(merge), result.into()));
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
            let result = |value| { 1 };
            |result| return result;
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
fn every_node_and_exit_has_one_identity() {
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
            let result = |&width, depth| { *width + depth };
            #[action("Use the far value.")]
            let result = |far| { far };
            |result| return result;
        }
    "#,
    );
    // RFC 0002 §5 shows every named flow input as an output of start; the
    // wildcard provides nothing.
    assert_eq!(
        topology.exit(ExitId::of(NodeId::Start)).provides,
        [ProducerId::FlowInput(0), ProducerId::FlowInput(1)]
    );
    let distributor = ExitId::of(NodeId::Block(0));
    assert!(topology.exit(distributor).provides.is_empty());
    assert_eq!(topology.leaving(distributor).count(), 2);
    for branch in 0..2 {
        let case = NodeId::Case { choice: 0, branch };
        assert_eq!(
            topology.exit(ExitId::of(case)).provides,
            [ProducerId::BlockOutput {
                block: 0,
                output: branch
            }]
        );
        assert_eq!(
            topology
                .incoming(Vertex::Node(case))
                .copied()
                .collect::<Vec<_>>(),
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
    // End has no exit.
    assert!(
        !topology
            .exits
            .iter()
            .any(|exit| exit.id.node == NodeId::Block(5))
    );
}

#[test]
fn a_merge_written_above_a_question_reaches_it_first() {
    let model = fixture(
        include_str!("../../../kaalang/tests/wire/behavior/a_branch_captures_a_merged_value.rs"),
        "a_branch_captures_a_merged_value",
    );
    let topology = &model.topology;
    let question = NodeId::Block(3);
    let connection = Connection {
        source: Source::Junction(0),
        destination: Destination::Node(question),
    };
    assert_eq!(merged(&model, 0), ["counted", "seen"]);
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
            let result = |value, doubled| { value + doubled };
            |result| return result;
        }
    "#,
    );
    let only_end = fixture(
        include_str!("../../../kaalang/tests/question/behavior/run_question.rs"),
        "run_question",
    )
    .topology;

    // The consumers of the merged wire, including the end reached through a
    // structural return in the second flow.
    for (topology, consumers) in [
        (&two_consumers, &[NodeId::Block(3), NodeId::Block(4)][..]),
        (&only_end, &[NodeId::Block(4)][..]),
    ] {
        for exit in &topology.exits {
            assert!(
                topology.leaving(exit.id).count() <= 1,
                "{:?} leaves more than once",
                exit.id
            );
        }
        assert_eq!(
            topology
                .junctions
                .iter()
                .filter(|junction| !junction.merges.is_empty())
                .count(),
            1
        );
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

/// A branch output with no alternative producers keeps its exit's single
/// connection however many blocks capture it. The first consumer is reached
/// from the exit and the rest through that consumer, so nothing routes around
/// the branch and RFC 0002 §7's reduction leaves one edge per exit.
#[test]
fn an_unmerged_branch_output_leaves_its_exit_once() {
    let topology = fixture(
        include_str!("../../../kaalang/tests/wire/behavior/branch_output_captured_twice.rs"),
        "branch_output_captured_twice",
    )
    .topology;

    for exit in &topology.exits {
        assert!(
            topology.leaving(exit.id).count() <= 1,
            "{:?} leaves more than once",
            exit.id
        );
    }
    // Both captures of `yes` sit in the branch, the second below the first.
    assert!(reaches(
        &topology,
        Vertex::Node(NodeId::Block(1)),
        Vertex::Node(NodeId::Block(2))
    ));
}

/// A branch-local block written above a merge reaches the junction like the
/// producers do, without becoming one of them.
#[test]
fn a_branch_local_block_reaches_the_merge_it_precedes() {
    let model = fixture(
        include_str!("../../../kaalang/tests/wire/behavior/local_work_before_a_wire_merge.rs"),
        "local_work_before_a_wire_merge",
    );
    // `selected` and `noted` arrive from the same two exits, so they share one
    // junction.
    assert_eq!(
        model
            .topology
            .junctions
            .iter()
            .filter(|junction| !junction.merges.is_empty())
            .count(),
        1
    );
    assert_eq!(merged(&model, 0), ["selected", "noted"]);

    for source in [NodeId::Block(1), NodeId::Block(2), NodeId::Block(3)] {
        assert!(
            reaches(&model.topology, Vertex::Node(source), Vertex::Junction(0)),
            "{source:?} does not reach the junction"
        );
    }
}
