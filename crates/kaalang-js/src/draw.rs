//! What [`crate::draw_flow`] answers: a flow's SVG and its hotspots.

use std::collections::BTreeMap;

use kaalang_compiler::{SourceError, SourceFile};
use kaalang_svg::{HotspotKind, RibbonWire};
use serde::Serialize;

use crate::utf16;

/// The answer. Each wire of a ribbon is listed once, in `wires`, since every
/// strip of a connection repeats its ribbon.
#[derive(Serialize)]
struct Drawn {
    svg: String,
    hotspots: Vec<Spot>,
    wires: Vec<Wire>,
}

/// A [`kaalang_svg::Hotspot`] with its spans as UTF-16 ranges.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Spot {
    kind: &'static str,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callee_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callee_end: Option<u32>,
    /// A connection's or a merge's ribbon, as positions in [`Drawn::wires`].
    #[serde(skip_serializing_if = "Option::is_none")]
    ribbon: Option<Vec<usize>>,
}

/// A [`kaalang_svg::RibbonWire`] with its binding as a UTF-16 range.
#[derive(Serialize)]
struct Wire {
    name: String,
    start: u32,
    end: u32,
}

/// The answer as JSON, `null` when no flow stands at `position`, or why the flow
/// cannot be drawn.
pub(crate) fn flow(
    text: &str,
    cfg: &str,
    position: usize,
    collapse_cycles: bool,
) -> Result<String, String> {
    let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
    let flow = match file.flow(position, collapse_cycles) {
        Err(SourceError::FlowNotFound(_)) => return Ok("null".into()),
        flow => flow.map_err(|error| error.to_string())?,
    };
    let (svg, hotspots) = kaalang_svg::render_flow(flow).map_err(|error| error.to_string())?;
    let offsets = utf16::Offsets::new(text);
    let range = |span| {
        let range = file.range(span);
        (offsets.at(range.start), offsets.at(range.end))
    };
    let mut wires = Vec::new();
    let mut listed = BTreeMap::new();
    // The positions of `ribbon` in `wires`, each listed at its first use. A
    // binding's range names its wire, so it names its entry too.
    let mut list = |ribbon: &[RibbonWire]| -> Vec<usize> {
        ribbon
            .iter()
            .map(|wire| {
                let (start, end) = range(wire.binding);
                *listed.entry((start, end)).or_insert_with(|| {
                    wires.push(Wire {
                        name: wire.name.clone(),
                        start,
                        end,
                    });
                    wires.len() - 1
                })
            })
            .collect()
    };
    let hotspots = hotspots
        .iter()
        .map(|hotspot| {
            let span = hotspot.span.map(range);
            let (kind, callee, ribbon) = match &hotspot.kind {
                HotspotKind::Node => ("node", None, None),
                HotspotKind::Call { callee } => ("call", Some(range(*callee)), None),
                HotspotKind::Wire => ("wire", None, None),
                HotspotKind::Connection { ribbon } => ("connection", None, Some(list(ribbon))),
                HotspotKind::Merge { ribbon } => ("merge", None, Some(list(ribbon))),
            };
            Spot {
                kind,
                left: hotspot.left,
                top: hotspot.top,
                right: hotspot.right,
                bottom: hotspot.bottom,
                start: span.map(|(start, _)| start),
                end: span.map(|(_, end)| end),
                callee_start: callee.map(|(start, _)| start),
                callee_end: callee.map(|(_, end)| end),
                ribbon,
            }
        })
        .collect();
    Ok(serde_json::to_string(&Drawn {
        svg,
        hotspots,
        wires,
    })
    .expect("a diagram serializes"))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::fixture::{SOURCE, unit_of};

    #[test]
    fn a_diagram_reaches_javascript_as_json() {
        let json: Value = serde_json::from_str(&flow(SOURCE, "", 0, false).unwrap()).unwrap();
        assert!(json["svg"].as_str().unwrap().starts_with("<svg "));
        let hotspots = json["hotspots"].as_array().unwrap();
        let call = hotspots.iter().find(|spot| spot["kind"] == "call").unwrap();
        assert_eq!(call["calleeStart"], unit_of("add("));
        assert_eq!(call["calleeEnd"], unit_of("add(") + 3);
    }

    #[test]
    fn only_a_call_names_a_callee() {
        let json: Value = serde_json::from_str(&flow(SOURCE, "", 0, false).unwrap()).unwrap();
        let hotspots = json["hotspots"].as_array().unwrap();
        assert!(hotspots.iter().any(|spot| spot["kind"] == "node"));
        for spot in hotspots.iter().filter(|spot| spot["kind"] != "call") {
            assert!(spot.get("calleeStart").is_none(), "{spot}");
            assert!(spot.get("calleeEnd").is_none(), "{spot}");
        }
    }

    /// A flow whose two branches merge `value`, and which carries `first` on
    /// three connections.
    const PICK: &str = "#[kaa]
fn pick(condition: bool, first: u8) -> u8 {
    #[kaa::question(\"Take the yes branch?\")]
    let (no, yes) = |condition| condition;

    #[kaa::action(\"Keep it.\")]
    let value = |no, first| first;

    #[kaa::action(\"Raise it.\")]
    let value = |yes, first| first + 1;

    |value| return value;
}
";

    /// The diagram of the first flow in `source`, as JSON.
    fn drawn(source: &str) -> Value {
        serde_json::from_str(&flow(source, "", 0, false).unwrap()).unwrap()
    }

    /// The ribbon of a hotspot of `drawn`, from the answer's table.
    fn listed<'a>(drawn: &'a Value, spot: &Value) -> Vec<&'a Value> {
        spot["ribbon"]
            .as_array()
            .unwrap()
            .iter()
            .map(|position| &drawn["wires"][usize::try_from(position.as_u64().unwrap()).unwrap()])
            .collect()
    }

    /// The hotspots of `drawn` of one `kind`.
    fn of_kind<'a>(drawn: &'a Value, kind: &str) -> Vec<&'a Value> {
        drawn["hotspots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|spot| spot["kind"] == kind)
            .collect()
    }

    #[test]
    fn a_connection_names_each_wire_with_its_binding() {
        let drawn = drawn(SOURCE);
        let first = unit_of("first: u8");
        let expected = serde_json::json!({"name": "first", "start": first, "end": first + 5});
        assert!(
            of_kind(&drawn, "connection")
                .iter()
                .any(|spot| listed(&drawn, spot) == [&expected]),
            "{drawn:#?}"
        );
    }

    #[test]
    fn a_merge_lists_its_ribbon_under_its_own_kind() {
        let drawn = drawn(PICK);
        let merge = of_kind(&drawn, "merge")[0];
        let names: Vec<&Value> = listed(&drawn, merge)
            .into_iter()
            .map(|wire| &wire["name"])
            .collect();
        assert_eq!(names, [&Value::from("value")]);
    }

    #[test]
    fn a_connection_has_no_range_of_its_own() {
        let drawn = drawn(SOURCE);
        for spot in of_kind(&drawn, "connection") {
            assert!(spot.get("start").is_none(), "{spot}");
        }
    }

    #[test]
    fn each_wire_is_listed_once() {
        let drawn = drawn(PICK);
        let wires: Vec<String> = drawn["wires"]
            .as_array()
            .unwrap()
            .iter()
            .map(Value::to_string)
            .collect();
        let unique: std::collections::BTreeSet<&String> = wires.iter().collect();
        assert_eq!(unique.len(), wires.len(), "{wires:#?}");
    }

    #[test]
    fn text_that_does_not_parse_says_so() {
        assert!(
            flow("fn", "", 0, false)
                .unwrap_err()
                .contains("could not parse")
        );
    }

    #[test]
    fn no_flow_at_a_position_draws_nothing() {
        assert_eq!(flow(SOURCE, "", 1, false), Ok("null".into()));
    }

    #[test]
    fn a_cfg_that_does_not_parse_says_so() {
        assert!(
            flow(SOURCE, "not a cfg", 0, false)
                .unwrap_err()
                .contains("not a cfg")
        );
    }
}
