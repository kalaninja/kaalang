use kaalang_compiler::topology::Vertex;
use kaalang_compiler::{ArrangementChecks, RunLine, SemanticModel};

fn model(source: &str) -> SemanticModel {
    kaalang_compiler::build(&syn::parse_str(source).expect("the flow parses"))
        .expect("the flow has an arrangement")
}

fn fixture(source: &str, name: &str) -> SemanticModel {
    let file = syn::parse_file(source).expect("the fixture parses");
    let function = kaalang_compiler::flows(&file.items)
        .into_iter()
        .find(|function| function.sig.ident == name)
        .expect("the fixture declares the flow");
    kaalang_compiler::build(&function).expect("the fixture has an arrangement")
}

#[test]
fn the_public_checks_reject_broken_placement_and_references() {
    let model = model(
        "fn probe(value: u8) -> u8 {
            #[action(\"Keep the value.\")]
            let kept = |value| value;
            |kept| return kept;
        }",
    );
    let checks = ArrangementChecks::new(&model.analysis.flow, &model.topology);
    checks.verify(&model.arrangement).unwrap();

    let vertex = model.topology.vertices[0];
    let mut missing = model.arrangement.clone();
    missing.rank.remove(&vertex);
    assert!(checks.verify(&missing).is_err());
    assert!(checks.normalize_geometry(missing, |_| Ok(())).is_err());

    let mut invalid = model.arrangement.clone();
    let rank = invalid.rank.remove(&vertex).unwrap();
    invalid.rank.insert(Vertex::Junction(usize::MAX), rank);
    assert!(checks.verify(&invalid).is_err());

    let mut broken = model.arrangement.clone();
    let deepest = broken
        .rank
        .iter()
        .max_by_key(|(_, rank)| **rank)
        .map(|(vertex, _)| *vertex)
        .unwrap();
    broken.rank.insert(deepest, 0);
    assert!(checks.verify_placement(&broken, |_| false).is_err());
}

#[test]
fn the_public_geometry_check_rejects_a_crossed_corridor() {
    let model = fixture(
        include_str!("../../kaalang/tests/wire/behavior/blocked_terminal_crossing.rs"),
        "blocked_terminal_crossing",
    );
    let checks = ArrangementChecks::new(&model.analysis.flow, &model.topology);
    checks
        .verify_geometry(&model.arrangement, |geometry| {
            assert_eq!(
                geometry.connections().len(),
                model.topology.connections.len()
            );
            Ok(())
        })
        .unwrap();

    let mut crossed = model.arrangement.clone();
    crossed.routes[10].runs[0].line = RunLine::Rank(0);
    crossed.routes[10].runs[0].enter = 1;
    crossed.routes[10].runs[0].exit = -1;
    let reason = checks.verify_geometry(&crossed, |_| Ok(())).unwrap_err();
    assert!(reason.contains("passes through"), "{reason}");
}
