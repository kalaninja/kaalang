use kaalang_compiler::{Arrangement, ArrangementChecks, RunLine, SemanticModel, topology::Vertex};
use kaalang_render::ArrangementVerifier;

/// One action between the flow's start and end: the smallest arrangement a
/// reference can be removed from.
const SERIAL_PROBE: &str = "fn probe(value: u8) -> u8 {
    #[action(\"Keep the value.\")]
    let kept = |value| value;
    |kept| return kept;
}";

fn model(source: &str) -> SemanticModel {
    kaalang_compiler::build(&syn::parse_str(source).expect("the flow parses"))
        .expect("the flow has an arrangement")
}

fn fixture(source: &str, name: &str) -> SemanticModel {
    kaalang_compiler::build(&kaalang_testing::corpus::flow_named(source, name))
        .expect("the fixture has an arrangement")
}

#[test]
fn the_public_verifier_rejects_missing_and_invalid_references() {
    let model = model(SERIAL_PROBE);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    verifier.normalize(model.arrangement.clone()).unwrap();

    let vertex = model.topology.vertices[0];
    let mut missing = model.arrangement.clone();
    missing.rank.remove(&vertex);
    assert!(verifier.normalize(missing).is_err());

    let mut invalid = model.arrangement.clone();
    let rank = invalid.rank.remove(&vertex).unwrap();
    invalid.rank.insert(Vertex::Junction(usize::MAX), rank);
    assert!(verifier.normalize(invalid).is_err());
}

#[test]
fn the_public_verifier_rejects_broken_order() {
    let model = model(SERIAL_PROBE);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    let mut broken = model.arrangement.clone();
    let deepest = broken
        .rank
        .iter()
        .max_by_key(|(_, rank)| **rank)
        .map(|(vertex, _)| *vertex)
        .unwrap();
    broken.rank.insert(deepest, 0);
    assert!(verifier.normalize(broken).is_err());
}

#[test]
fn the_public_verifier_rejects_overflowing_coordinates() {
    let model = model(SERIAL_PROBE);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    let mut broken = model.arrangement.clone();
    let start = Vertex::Node(kaalang_compiler::topology::NodeId::Start);
    broken.column.insert(start, 1);
    *broken.exit_offset.values_mut().next().unwrap() = i32::MAX;
    assert!(
        ArrangementChecks::new(&model.analysis.flow, &model.topology)
            .verify(&broken)
            .is_err()
    );
    assert!(verifier.normalize(broken).is_err());
}

#[test]
fn the_public_verifier_rejects_an_overflowing_contour_lane() {
    let source = kaalang_testing::shapes::cycle_routes(&["repeat", "leave"]);
    let model = model(&source);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    let mut broken = model.arrangement.clone();
    broken.contours[0].lane = i32::MAX as usize;
    assert!(
        ArrangementChecks::new(&model.analysis.flow, &model.topology)
            .verify(&broken)
            .is_err()
    );
    assert!(verifier.normalize(broken).is_err());
}

#[test]
fn the_public_verifiers_reject_unrepresentable_grid_spacing() {
    let source = kaalang_testing::shapes::cycle_routes(&["repeat", "leave"]);
    let model = model(&source);
    let checks = ArrangementChecks::new(&model.analysis.flow, &model.topology);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    let mutations: [fn(&mut Arrangement); 3] = [
        |built| built.gap_lanes[0] = usize::MAX,
        |built| built.gap_lanes[0] = (i32::MAX - 2) as usize,
        |built| built.contours[0].column = i32::MAX,
    ];
    for mutate in mutations {
        let mut broken = model.arrangement.clone();
        mutate(&mut broken);
        assert!(checks.verify(&broken).is_err());
        assert!(verifier.normalize(broken).is_err());
    }
}

#[test]
fn the_public_verifier_rejects_a_crossed_corridor() {
    let mut model = fixture(
        include_str!("../../kaalang/tests/wire/behavior/blocked_terminal_crossing.rs"),
        "blocked_terminal_crossing",
    );
    kaalang_render::compact_arrangement(&mut model);
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    verifier.normalize(model.arrangement.clone()).unwrap();

    let mut crossed = model.arrangement.clone();
    crossed.routes[10].runs[0].line = RunLine::Rank(0);
    crossed.routes[10].runs[0].enter = 1;
    crossed.routes[10].runs[0].exit = -1;
    let reason = verifier.normalize(crossed).unwrap_err();
    assert!(reason.contains("passes through"), "{reason}");
}

#[test]
fn a_lifted_completion_is_a_renderer_exception_not_a_construction_result() {
    let mut model = fixture(
        include_str!("../../kaalang/tests/gallery/sorting/bubble_sort.rs"),
        "bubble_sort",
    );
    kaalang_render::compact_arrangement(&mut model);

    ArrangementVerifier::new(&model.analysis.flow, &model.topology)
        .normalize(model.arrangement.clone())
        .unwrap();
    let reason = ArrangementChecks::new(&model.analysis.flow, &model.topology)
        .verify(&model.arrangement)
        .expect_err("construction keeps strict placement precedence");
    assert!(reason.contains("placement precedence"), "{reason}");
}
