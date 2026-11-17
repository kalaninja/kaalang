//! What [`crate::draw_flow`] answers: a flow's SVG and its hotspots.

use kaalang_compiler::SourceFile;
use kaalang_svg::HotspotKind;
use serde::Serialize;

use crate::utf16;

/// The answer.
#[derive(Serialize)]
struct Drawn {
    svg: String,
    hotspots: Vec<Spot>,
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
    start: u32,
    end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    callee_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callee_end: Option<u32>,
}

/// The answer as JSON, or why the flow cannot be drawn.
pub(crate) fn flow(
    text: &str,
    cfg: &str,
    position: usize,
    collapse_cycles: bool,
) -> Result<String, String> {
    let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
    let flow = file
        .flow(position, collapse_cycles)
        .map_err(|error| error.to_string())?;
    let (svg, hotspots) = kaalang_svg::render_flow(flow).map_err(|error| error.to_string())?;
    let offsets = utf16::Offsets::new(text);
    let range = |span| {
        let range = file.range(span);
        (offsets.at(range.start), offsets.at(range.end))
    };
    let hotspots = hotspots
        .iter()
        .map(|hotspot| {
            let (start, end) = range(hotspot.span);
            let (kind, callee) = match hotspot.kind {
                HotspotKind::Node => ("node", None),
                HotspotKind::Call { callee } => ("call", Some(range(callee))),
                HotspotKind::Wire => ("wire", None),
            };
            Spot {
                kind,
                left: hotspot.left,
                top: hotspot.top,
                right: hotspot.right,
                bottom: hotspot.bottom,
                start,
                end,
                callee_start: callee.map(|(start, _)| start),
                callee_end: callee.map(|(_, end)| end),
            }
        })
        .collect();
    Ok(serde_json::to_string(&Drawn { svg, hotspots }).expect("a diagram serializes"))
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

    #[test]
    fn text_that_does_not_parse_says_so() {
        assert!(
            flow("fn", "", 0, false)
                .unwrap_err()
                .contains("could not parse")
        );
    }

    #[test]
    fn a_missing_flow_names_its_position() {
        assert!(
            flow(SOURCE, "", 1, false)
                .unwrap_err()
                .contains("position 1")
        );
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
