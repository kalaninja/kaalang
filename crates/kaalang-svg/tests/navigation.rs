//! What an editor asks of a flow as it has the source: what each part of the
//! diagram stands for there, whatever the file's line endings.
//!
//! Hotspots are checked against the SVG they describe: what a diagram draws is
//! read back from its text, so a hotspot that drifts from the drawing fails.

use std::ops::Range;

use kaalang_compiler::{SourceFile, Target};
use kaalang_svg::{HotspotKind, render_flow};
use kaalang_testing::corpus::{self, Suite};

/// Font size of a wire label, which lifts a name's hotspot above its baseline.
const WIRE_FONT: i32 = 12;
/// How far a connection's strip reaches either side of its line, and a merge's
/// square from its centre.
const REACH: i32 = 12;

/// The numbers of one `name="…"` attribute after `name`, such as the two of
/// `translate(60 24)`.
fn numbers(line: &str, name: &str) -> Vec<i32> {
    let (_, value) = line
        .split_once(&format!("{name}=\""))
        .expect("the element has the attribute");
    let (value, _) = value.split_once('"').expect("the attribute is quoted");
    value
        .split(|character: char| !character.is_ascii_digit() && character != '-')
        .filter(|number| !number.is_empty())
        .map(|number| number.parse().expect("a whole number"))
        .collect()
}

/// What an SVG draws, in drawing order and on the whole canvas: the centre of
/// every node but a merge, the rectangle of every call node and of every cycle
/// boundary, the anchor and the lines of every wire label, the centre of every
/// merge, and every segment of every connection.
struct Drawn {
    width: i32,
    height: i32,
    nodes: Vec<(i32, i32)>,
    calls: Vec<(i32, i32, i32, i32)>,
    boundaries: Vec<(i32, i32, i32, i32)>,
    labels: Vec<((i32, i32), Vec<String>)>,
    merges: Vec<(i32, i32)>,
    segments: Vec<((i32, i32), (i32, i32))>,
}

fn drawn(svg: &str) -> Drawn {
    let mut drawn = Drawn {
        width: numbers(svg, "width")[0],
        height: numbers(svg, "height")[0],
        nodes: Vec::new(),
        calls: Vec::new(),
        boundaries: Vec::new(),
        labels: Vec::new(),
        merges: Vec::new(),
        segments: Vec::new(),
    };
    // A staged diagram draws each part inside a translated group.
    let mut part = (0, 0);
    let mut in_connection = false;
    let mut lines = svg.lines();
    while let Some(line) = lines.next() {
        if line.contains("<path class=\"connection\" d=\"") {
            in_connection = true;
            continue;
        }
        if in_connection {
            if line.trim_start().starts_with('M') {
                let numbers: Vec<i32> = line
                    .split_whitespace()
                    .filter_map(|token| token.parse().ok())
                    .collect();
                let points: Vec<(i32, i32)> = numbers
                    .chunks(2)
                    .map(|point| (part.0 + point[0], part.1 + point[1]))
                    .collect();
                drawn
                    .segments
                    .extend(points.windows(2).map(|pair| (pair[0], pair[1])));
                continue;
            }
            in_connection = false;
        }
        if line.contains("<g class=\"node merge\"") {
            let center = numbers(line, "transform");
            drawn.merges.push((part.0 + center[0], part.1 + center[1]));
        }
        if line.starts_with("<g transform=\"translate(") {
            let offset = numbers(line, "transform");
            part = (offset[0], offset[1]);
        } else if line.contains("<rect class=\"cycle-boundary\"") {
            let (x, y) = (
                part.0 + numbers(line, " x")[0],
                part.1 + numbers(line, " y")[0],
            );
            let (width, height) = (numbers(line, "width")[0], numbers(line, "height")[0]);
            drawn.boundaries.push((x, y, x + width, y + height));
        } else if line.contains("<g class=\"node ") && !line.contains("node merge") {
            let center = numbers(line, "transform");
            drawn.nodes.push((part.0 + center[0], part.1 + center[1]));
        }
        if line.contains("<g class=\"node call\"") {
            let center = numbers(line, "transform");
            let shape = lines
                .find(|line| line.contains("class=\"node-shape\""))
                .expect("a call node draws its shape");
            let (x, y) = (part.0 + center[0], part.1 + center[1]);
            let (half_width, half_height) = (-numbers(shape, "x")[0], -numbers(shape, "y")[0]);
            drawn.calls.push((
                x - half_width,
                y - half_height,
                x + half_width,
                y + half_height,
            ));
        } else if line.contains("<text class=\"connection-label code\"") {
            let at = (
                part.0 + numbers(line, " x")[0],
                part.1 + numbers(line, " y")[0],
            );
            let text = line.split("<tspan").skip(1).map(|line| {
                let (_, line) = line.split_once('>').expect("a line opens");
                let (line, _) = line.split_once("</tspan>").expect("a line closes");
                line.replace("&amp;", "&")
            });
            drawn.labels.push((at, text.collect()));
        }
    }
    drawn
}

/// A wire name without its capture form or `mut`.
fn bare(name: &str) -> &str {
    let name = name.strip_prefix('&').unwrap_or(name);
    name.strip_prefix("mut ").unwrap_or(name)
}

/// What a hotspot shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Node,
    Call,
    Wire,
    Connection,
    Merge,
}

/// A hotspot with the bytes of the text its spans cover.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Located {
    kind: Kind,
    /// A call's callee name.
    callee: Option<Range<usize>>,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    source: Range<usize>,
    /// A ribbon: each wire's name, and the bytes where its producer binds it.
    ribbon: Vec<(String, Range<usize>)>,
}

/// Checks that every node but a merge, and every boundary, has one hotspot on
/// it, and that no node hotspot lies on nothing drawn.
fn assert_every_node_has_one_hotspot(what: &str, drawn: &Drawn, hotspots: &[Located]) {
    let mut unmatched: Vec<&Located> = hotspots
        .iter()
        .filter(|hotspot| !matches!(hotspot.kind, Kind::Wire | Kind::Connection | Kind::Merge))
        .collect();
    let mut take = |matches: &dyn Fn(&Located) -> bool, what_on: String| {
        let at = unmatched
            .iter()
            .position(|hotspot| matches(hotspot))
            .unwrap_or_else(|| panic!("{what}: no hotspot on {what_on}"));
        unmatched.remove(at);
    };
    for &bounds in &drawn.boundaries {
        let on = move |hotspot: &Located| {
            (hotspot.left, hotspot.top, hotspot.right, hotspot.bottom) == bounds
        };
        take(&on, format!("the boundary {bounds:?}"));
    }
    for &center in &drawn.nodes {
        let on = move |hotspot: &Located| {
            (
                i32::midpoint(hotspot.left, hotspot.right),
                i32::midpoint(hotspot.top, hotspot.bottom),
            ) == center
        };
        take(&on, format!("the node at {center:?}"));
    }
    assert!(
        unmatched.is_empty(),
        "{what}: hotspots on nothing drawn: {unmatched:?}"
    );
}

/// Checks that one strip lies along every segment of every connection, ending
/// where the segment meets a node and reaching `REACH` past any other end, that
/// one square sits on every merge, and that each wire they list is named where
/// its producer binds it.
fn assert_ribbons_match_the_drawing(what: &str, source: &str, drawn: &Drawn, hotspots: &[Located]) {
    let bounds = |hotspot: &&Located| (hotspot.left, hotspot.top, hotspot.right, hotspot.bottom);
    let carrying: Vec<&Located> = hotspots
        .iter()
        .filter(|hotspot| matches!(hotspot.kind, Kind::Connection | Kind::Merge))
        .collect();
    let (squares, strips): (Vec<&Located>, Vec<&Located>) = carrying
        .iter()
        .partition(|hotspot| hotspot.kind == Kind::Merge);
    let nodes: Vec<_> = hotspots
        .iter()
        .filter(|hotspot| matches!(hotspot.kind, Kind::Node | Kind::Call))
        .map(|hotspot| bounds(&hotspot))
        .filter(|node| !drawn.boundaries.contains(node))
        .collect();
    let reach = |(x, y): (i32, i32)| {
        let at_node = nodes
            .iter()
            .any(|&(left, top, right, bottom)| left <= x && x <= right && top <= y && y <= bottom);
        if at_node { 0 } else { REACH }
    };
    let mut strips: Vec<_> = strips.iter().map(bounds).collect();
    let mut segments: Vec<_> = drawn
        .segments
        .iter()
        .filter(|(from, to)| from != to)
        .map(|&(from, to)| {
            let (low, high) = (from.min(to), from.max(to));
            if low.1 == high.1 {
                (
                    low.0 - reach(low),
                    low.1 - REACH,
                    high.0 + reach(high),
                    high.1 + REACH,
                )
            } else {
                (
                    low.0 - REACH,
                    low.1 - reach(low),
                    high.0 + REACH,
                    high.1 + reach(high),
                )
            }
        })
        .collect();
    strips.sort_unstable();
    segments.sort_unstable();
    assert_eq!(strips, segments, "{what}: one strip along each segment");
    let mut squares: Vec<_> = squares.iter().map(bounds).collect();
    let mut merges: Vec<_> = drawn
        .merges
        .iter()
        .map(|&(x, y)| (x - REACH, y - REACH, x + REACH, y + REACH))
        .collect();
    squares.sort_unstable();
    merges.sort_unstable();
    assert_eq!(squares, merges, "{what}: one square on each merge");
    for (name, binding) in carrying.iter().flat_map(|hotspot| &hotspot.ribbon) {
        let bound = &source[binding.clone()];
        assert_eq!(
            bound.strip_prefix("r#").unwrap_or(bound),
            name,
            "{what}: a connection or a merge names each wire where it binds"
        );
    }
}

/// Checks every hotspot of one diagram against what its SVG draws.
fn assert_hotspots_match_the_drawing(what: &str, source: &str, svg: &str, hotspots: &[Located]) {
    let drawn = drawn(svg);
    for hotspot in hotspots {
        assert!(
            0 <= hotspot.left
                && hotspot.left < hotspot.right
                && hotspot.right <= drawn.width
                && 0 <= hotspot.top
                && hotspot.top < hotspot.bottom
                && hotspot.bottom <= drawn.height,
            "{what}: {hotspot:?} leaves the canvas"
        );
    }

    assert_ribbons_match_the_drawing(what, source, &drawn, hotspots);

    let calls: Vec<&Located> = hotspots
        .iter()
        .filter(|hotspot| hotspot.kind == Kind::Call)
        .collect();
    assert_eq!(calls.len(), drawn.calls.len(), "{what}: call hotspots");
    for (hotspot, bounds) in calls.iter().zip(&drawn.calls) {
        assert_eq!(
            (hotspot.left, hotspot.top, hotspot.right, hotspot.bottom),
            *bounds,
            "{what}: a call hotspot lies on its node"
        );
        let callee = hotspot.callee.clone().expect("a call names its callee");
        let name = &source[callee.clone()];
        assert!(
            syn::parse_str::<syn::Ident>(name).is_ok(),
            "{what}: `{name}` names a callee"
        );
        assert!(
            source[callee.end..].trim_start().starts_with(['(', ':']),
            "{what}: `{name}` ends the callee path"
        );
    }

    assert_every_node_has_one_hotspot(what, &drawn, hotspots);

    let mut wires = hotspots.iter().filter(|hotspot| hotspot.kind == Kind::Wire);
    // What separates two names on one line is the same everywhere.
    let mut separator = None;
    for ((x, y), lines) in &drawn.labels {
        let mut first = true;
        let mut name_start = 0;
        // The line and right edge of the last name drawn.
        let mut last: Option<(usize, i32)> = None;
        for name in lines.concat().split(", ") {
            let name_end = name_start + name.len();
            let mut line_start = 0;
            for (line, text) in lines.iter().enumerate() {
                let line_end = line_start + text.len();
                if name != "()" && name_start.max(line_start) < name_end.min(line_end) {
                    let hotspot = wires
                        .next()
                        .unwrap_or_else(|| panic!("{what}: `{name}` has no hotspot"));
                    let bound = &source[hotspot.source.clone()];
                    assert_eq!(
                        bound.strip_prefix("r#").unwrap_or(bound),
                        bare(name),
                        "{what}: a wire hotspot names its binding"
                    );
                    assert!(hotspot.left >= *x, "{what}: `{name}` starts in its label");
                    if first {
                        assert_eq!(
                            (hotspot.left, hotspot.top),
                            (*x, *y - WIRE_FONT),
                            "{what}: `{name}` opens its label"
                        );
                    }
                    if line > 0 {
                        assert!(hotspot.top > *y - WIRE_FONT, "{what}: `{name}` wraps down");
                    }
                    if name_start < line_start {
                        assert_eq!(
                            hotspot.left, *x,
                            "{what}: `{name}` goes on at a line's start"
                        );
                    }
                    if let Some((last_line, right)) = last
                        && last_line == line
                        && name_start >= line_start
                    {
                        let gap = hotspot.left - right;
                        assert!(gap > 0, "{what}: `{name}` overlaps the name before it");
                        // Each edge rounds on its own, so a gap may differ by one.
                        assert!(
                            separator.get_or_insert(gap).abs_diff(gap) <= 1,
                            "{what}: `{name}` stands {gap} apart, unlike the others"
                        );
                    }
                    last = Some((line, hotspot.right));
                }
                first = false;
                line_start = line_end;
            }
            name_start = name_end + ", ".len();
        }
    }
    assert_eq!(wires.next(), None, "{what}: a hotspot without a wire name");
}

/// The SVG and the hotspots of the flow at `position` in `source`, read for no
/// target, with each cycle as one node when `collapse_cycles` is set.
fn draw(
    source: &str,
    position: usize,
    collapse_cycles: bool,
) -> Result<(String, Vec<Located>), String> {
    let file = SourceFile::parse(source, Target::EMPTY).map_err(|error| error.to_string())?;
    let flow = file
        .flow(position, collapse_cycles)
        .map_err(|error| error.to_string())?;
    let (svg, hotspots) = render_flow(flow).map_err(|error| error.to_string())?;
    let located = hotspots
        .iter()
        .map(|hotspot| Located {
            kind: match hotspot.kind {
                HotspotKind::Node => Kind::Node,
                HotspotKind::Call { .. } => Kind::Call,
                HotspotKind::Wire => Kind::Wire,
                HotspotKind::Connection { .. } => Kind::Connection,
                HotspotKind::Merge { .. } => Kind::Merge,
            },
            callee: match hotspot.kind {
                HotspotKind::Call { callee } => Some(file.range(callee)),
                _ => None,
            },
            ribbon: match &hotspot.kind {
                HotspotKind::Connection { ribbon } | HotspotKind::Merge { ribbon } => ribbon
                    .iter()
                    .map(|wire| (wire.name.clone(), file.range(wire.binding)))
                    .collect(),
                _ => Vec::new(),
            },
            left: hotspot.left,
            top: hotspot.top,
            right: hotspot.right,
            bottom: hotspot.bottom,
            source: hotspot.span.map_or(0..0, |span| file.range(span)),
        })
        .collect();
    Ok((svg, located))
}

#[test]
fn every_fixture_diagram_lists_the_hotspots_it_draws() {
    let mut calls = 0;
    let mut wires = 0;
    for diagram in corpus::diagrams(Suite::Ordinary) {
        let (what, source) = (diagram.to_string(), &diagram.source);
        let (svg, hotspots) = draw(source, diagram.position, diagram.collapse_cycles)
            .unwrap_or_else(|error| panic!("{what}: {error}"));
        assert_hotspots_match_the_drawing(&what, source, &svg, &hotspots);
        let count = |kind| {
            hotspots
                .iter()
                .filter(|hotspot| hotspot.kind == kind)
                .count()
        };
        calls += count(Kind::Call);
        wires += count(Kind::Wire);
    }
    // The corpus must exercise both kinds for the checks above to mean anything.
    assert!(calls >= 10, "{calls} call hotspots");
    assert!(wires > 1000, "{wires} wire hotspots");
}

#[test]
fn crlf_inside_a_description_draws_the_same_diagram() {
    // A raw string keeps its line endings as written, and a cooked one is
    // read through its escapes.
    let source = r##"#[kaa]
fn describe(
    seed: u8, /* a comment
    over two lines */
) -> u8 {
    #[kaa::action(r#"Take the seed
and keep it.

A second paragraph."#)]
    let kept = |seed| { seed };

    #[kaa::action("Double it \
        on one line.

A second paragraph.")]
    let end = |kept| { kept * 2 };

    |end| return end;
}
"##;
    let crlf = source.replace('\n', "\r\n");
    assert_eq!(
        draw(&crlf, 0, false),
        draw(source, 0, false).map(|(svg, hotspots)| {
            let line = |offset: usize| source[..offset].matches('\n').count();
            let moved =
                |range: Range<usize>| range.start + line(range.start)..range.end + line(range.end);
            let moved = hotspots.into_iter().map(|hotspot| Located {
                source: moved(hotspot.source),
                ribbon: hotspot
                    .ribbon
                    .into_iter()
                    .map(|(name, binding)| (name, moved(binding)))
                    .collect(),
                ..hotspot
            });
            (svg, moved.collect())
        })
    );
}

#[test]
fn crlf_line_endings_draw_the_same_diagram() {
    for diagram in corpus::diagrams(Suite::Ordinary) {
        let (what, source) = (diagram.to_string(), &diagram.source);
        let (position, collapse_cycles) = (diagram.position, diagram.collapse_cycles);
        let crlf = source.replace('\n', "\r\n");
        let (svg, hotspots) = draw(source, position, collapse_cycles)
            .unwrap_or_else(|error| panic!("{what}: {error}"));
        let (crlf_svg, crlf_hotspots) = draw(&crlf, position, collapse_cycles)
            .unwrap_or_else(|error| panic!("{what}, CRLF: {error}"));
        assert_eq!(svg, crlf_svg, "{what}");
        assert_eq!(hotspots.len(), crlf_hotspots.len(), "{what}");
        for (hotspot, crlf_hotspot) in hotspots.iter().zip(&crlf_hotspots) {
            assert_eq!(
                (
                    hotspot.kind,
                    hotspot.left,
                    hotspot.top,
                    hotspot.right,
                    hotspot.bottom,
                    source[hotspot.source.clone()].to_owned()
                ),
                (
                    crlf_hotspot.kind,
                    crlf_hotspot.left,
                    crlf_hotspot.top,
                    crlf_hotspot.right,
                    crlf_hotspot.bottom,
                    // A block's code spans lines, and keeps its line endings.
                    crlf[crlf_hotspot.source.clone()].replace("\r\n", "\n")
                ),
                "{what}"
            );
        }
    }
}

const SOURCE: &str = r#"#[kaa]
fn settle(first: u8, mut r#type: u8) -> u8 {
    #[kaa::call("Add both.")]
    let sum = |first, r#type| math::add::<u8>(first, r#type);

    #[kaa::call]
    let end = |sum| <u8 as Double>::double(sum);

    |end| return end;
}
"#;

/// The byte range of the first `text` in `source`.
fn first(source: &str, text: &str) -> Range<usize> {
    let start = source.find(text).expect("the source has the text");
    start..start + text.len()
}

/// The byte range of `name` inside the first `context` in `source`.
fn within(source: &str, context: &str, name: &str) -> Range<usize> {
    let context = first(source, context);
    let start = context.start + source[context].find(name).expect("the context names it");
    start..start + name.len()
}

/// Where the wire hotspots spelled `name` lead, each place once, in source order.
fn wire_bindings(source: &str, hotspots: &[Located], name: &str) -> Vec<Range<usize>> {
    let mut bindings: Vec<_> = hotspots
        .iter()
        .filter(|hotspot| hotspot.kind == Kind::Wire && source[hotspot.source.clone()] == *name)
        .map(|hotspot| hotspot.source.clone())
        .collect();
    bindings.sort_by_key(|binding| binding.start);
    bindings.dedup();
    bindings
}

#[test]
fn a_call_hotspot_names_the_last_segment_of_its_callee() {
    let (_, hotspots) = draw(SOURCE, 0, false).unwrap();
    let callees: Vec<_> = hotspots
        .iter()
        .filter_map(|hotspot| hotspot.callee.clone())
        .collect();
    assert_eq!(
        callees,
        [first(SOURCE, "add"), first(SOURCE, "double")],
        "neither the module, the generic arguments, nor the qualified type"
    );
}

#[test]
fn a_wire_hotspot_names_the_binding_of_its_producer() {
    let (_, hotspots) = draw(SOURCE, 0, false).unwrap();
    let mut bindings: Vec<_> = hotspots
        .iter()
        .filter(|hotspot| hotspot.kind == Kind::Wire)
        .map(|hotspot| hotspot.source.clone())
        .collect();
    bindings.sort_by_key(|binding| binding.start);
    bindings.dedup();
    assert_eq!(
        bindings,
        [
            first(SOURCE, "first"),
            // Without its `mut`, which is not part of the name.
            first(SOURCE, "r#type"),
            first(SOURCE, "sum"),
            first(SOURCE, "end"),
        ],
        "a parameter in the signature, an output in its pattern"
    );
}

#[test]
fn a_returned_wire_keeps_its_hotspot_under_another_raw_spelling() {
    let source = "#[kaa]\nfn raw(first: u8, second: u8) -> (u8, u8) {\n    |r#first, second| return (second, first);\n}\n";
    let (_, hotspots) = draw(source, 0, false).unwrap();
    let named = |text| {
        let binding = first(source, text);
        hotspots
            .iter()
            .filter(|hotspot| hotspot.source == binding)
            .count()
    };
    // Each name stands in the hand-over below start and in the label above end.
    assert_eq!((named("first"), named("second")), (2, 2));
}

#[test]
fn a_wire_name_that_wraps_has_a_hotspot_on_each_line() {
    let long = "a_wire_name_far_too_long_for_the_room_one_label_has_beside_its_connection";
    let source = SOURCE.replace("sum", long);
    let (svg, hotspots) = draw(&source, 0, false).unwrap();
    // The check reads a hotspot for each line a name is drawn on.
    assert_hotspots_match_the_drawing("a wrapped name", &source, &svg, &hotspots);
    assert!(
        drawn(&svg).labels.iter().any(|(_, lines)| {
            lines.concat().contains(long) && lines.iter().all(|line| !line.contains(long))
        }),
        "no label wraps the name"
    );
}

/// A staged flow whose second part calls, and captures a parameter it shares
/// with preparation.
const STAGED: &str = r#"#[kaa]
fn staged(seed: u8) -> u8 {
    #[kaa::action("Prepare the first value.")]
    let go = |seed| seed + 1;

    #[kaa::stage("Use the value.")]
    let finish = |go| {
        #[kaa::call]
        let finish = |go, seed| math::add(go, seed);
    };

    #[kaa::stage("Return it.")]
    |finish| {
        |finish| return finish;
    };
}
"#;

#[test]
fn a_call_in_a_stage_part_has_its_hotspot() {
    let (svg, hotspots) = draw(STAGED, 0, false).unwrap();
    assert_hotspots_match_the_drawing("a staged call", STAGED, &svg, &hotspots);
    let calls: Vec<_> = hotspots
        .iter()
        .filter_map(|hotspot| hotspot.callee.clone())
        .collect();
    assert_eq!(calls, [first(STAGED, "add")]);
}

#[test]
fn a_stage_entry_leads_to_where_its_stage_header_names_it() {
    let (_, hotspots) = draw(STAGED, 0, false).unwrap();
    assert_eq!(
        wire_bindings(STAGED, &hotspots, "go"),
        [within(STAGED, "let finish = |go| {", "go")]
    );
    assert_eq!(
        wire_bindings(STAGED, &hotspots, "finish"),
        [within(STAGED, "|finish| {", "finish")]
    );
}

#[test]
fn a_wire_shared_with_preparation_leads_to_its_binding_there() {
    let (_, hotspots) = draw(STAGED, 0, false).unwrap();
    assert_eq!(
        wire_bindings(STAGED, &hotspots, "seed"),
        [first(STAGED, "seed")]
    );
}

#[test]
fn a_merged_wire_leads_to_its_first_producer() {
    let source = r#"#[kaa]
fn merged(condition: bool, mode: bool) -> usize {
    #[kaa::question("Provide the value directly?")]
    let (mut selected, choose) = |condition| condition;

    #[kaa::choice("Choose the alternative producer.")]
    #[kaa::case("Provide the value directly.")]
    #[kaa::case("Produce it in an action.")]
    let (mut selected, prepare) = |choose, mode| match mode {
        true => (),
        false => (),
    };

    #[kaa::action("Produce the final alternative.")]
    let mut selected = |prepare| {};

    #[kaa::action("Mutate only after the branch outputs merge.")]
    let end = |&mut selected| {
        *selected = ();
        1
    };

    |end| return end;
}
"#;
    let (_, hotspots) = draw(source, 0, false).unwrap();
    assert_eq!(
        wire_bindings(source, &hotspots, "selected"),
        [first(source, "selected")]
    );
}

#[test]
fn a_receiver_leads_to_its_self() {
    let source = r#"struct Counter {
    total: u8,
}

impl Counter {
    #[kaa]
    fn total(&self) -> u8 {
        #[kaa::action("Read the total.")]
        let total = |&self| self.total;

        |total| return total;
    }
}
"#;
    let (_, hotspots) = draw(source, 0, false).unwrap();
    assert_eq!(
        wire_bindings(source, &hotspots, "self"),
        [within(source, "(&self)", "self")]
    );
}

#[test]
fn an_item_leads_to_its_for_pattern() {
    let source = r#"#[kaa]
fn sum(values: &[i64]) -> i64 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add every value to the total.")]
    |values| for value in values {
        #[kaa::action("Add the value.")]
        |value, &mut total| *total += *value;
    };

    |total| return total;
}
"#;
    let (_, hotspots) = draw(source, 0, false).unwrap();
    assert_eq!(
        wire_bindings(source, &hotspots, "value"),
        [within(source, "for value in", "value")]
    );
}

#[test]
fn a_cycle_output_leads_to_its_producer_inside_the_cycle_and_to_the_cycle_outside() {
    let source = r#"#[kaa]
fn count_to(limit: usize) -> usize {
    #[kaa::action("Initialize the counter.")]
    let mut count = || 0;

    #[kaa::cycle("Count to the limit.")]
    let leave = loop {
        #[kaa::question("Is the counter below the limit?")]
        let (again, leave) = |&count, &limit| *count < *limit;

        #[kaa::action("Increment the counter.")]
        |again, &mut count| *count += 1;

        |again| continue;
    };

    |leave, count| return count;
}
"#;
    // Expanded, the cycle's result is named only inside, where the question
    // produces it; collapsed, on the cycle node's exit.
    let (_, expanded) = draw(source, 0, false).unwrap();
    assert_eq!(
        wire_bindings(source, &expanded, "leave"),
        [within(source, "let (again, leave)", "leave")]
    );
    let (_, collapsed) = draw(source, 0, true).unwrap();
    assert_eq!(
        wire_bindings(source, &collapsed, "leave"),
        [within(source, "let leave = loop", "leave")]
    );
}

/// The source under each code hotspot, in source order.
fn code<'a>(source: &'a str, hotspots: &[Located]) -> Vec<&'a str> {
    let mut code: Vec<_> = hotspots
        .iter()
        .filter(|hotspot| hotspot.kind == Kind::Node)
        .map(|hotspot| hotspot.source.clone())
        .collect();
    code.sort_by_key(|range| range.start);
    code.into_iter().map(|range| &source[range]).collect()
}

#[test]
fn a_node_leads_to_where_its_block_starts() {
    let source = include_str!("../../kaalang/tests/flow/capture/mutate_merged_branch_outputs.rs");
    let (_, hotspots) = draw(source, 0, false).unwrap();
    assert_eq!(
        code(source, &hotspots),
        [
            "mutate_merged_branch_outputs",
            r#"#[kaa::question("Provide the value directly?")]"#,
            r#"#[kaa::choice("Choose the alternative producer.")]"#,
            "true => (),",
            "false => (),",
            r#"#[kaa::action("Produce the final alternative.")]"#,
            r#"#[kaa::action("Mutate only after the branch outputs merge.")]"#,
            "return end",
        ],
        "start, a question, a select and its cases, actions, and end"
    );
}

#[test]
fn a_cycle_its_boundary_and_its_caps_lead_to_the_cycle() {
    let looped = include_str!("../../kaalang/tests/flow/cycle/count_to.rs");
    let (_, expanded) = draw(looped, 0, false).unwrap();
    let (_, collapsed) = draw(looped, 0, true).unwrap();
    let iterated = include_str!("../../kaalang/tests/flow/cycle/for_each_value.rs");
    let (_, capped) = draw(iterated, 0, false).unwrap();
    let cycle = r#"#[kaa::cycle("Count to the limit.")]"#;
    let body = [
        r#"#[kaa::question("Is the counter below the limit?")]"#,
        r#"#[kaa::action("Increment the counter.")]"#,
    ];
    let around = ["count_to", r#"#[kaa::action("Initialize the counter.")]"#];
    assert_eq!(
        code(looped, &expanded),
        [&around[..], &[cycle], &body[..], &["return count"]].concat()
    );
    let leads_to = |text: &str| {
        expanded
            .iter()
            .find(|hotspot| hotspot.kind == Kind::Node && &looped[hotspot.source.clone()] == text)
            .expect("a hotspot leads there")
    };
    let (boundary, inner) = (leads_to(cycle), leads_to(body[0]));
    assert!(
        boundary.left < inner.left
            && boundary.top < inner.top
            && inner.right < boundary.right
            && inner.bottom < boundary.bottom,
        "the boundary's hotspot holds its body's"
    );
    assert_eq!(
        code(looped, &collapsed),
        [&around[..], &[cycle, "return count"]].concat()
    );
    let each = r#"#[kaa::cycle("Add every value to the total.")]"#;
    assert_eq!(
        code(iterated, &capped),
        [
            "for_each_value",
            r#"#[kaa::action("Start with a total of zero.")]"#,
            // The for-entry and the for-end.
            each,
            each,
            r#"#[kaa::action("Add the value.")]"#,
            "return total",
        ]
    );
}

#[test]
fn a_stage_entry_and_a_transition_draw_their_stage() {
    let (_, hotspots) = draw(STAGED, 0, false).unwrap();
    let first = r#"#[kaa::stage("Use the value.")]"#;
    let second = r#"#[kaa::stage("Return it.")]"#;
    assert_eq!(
        code(STAGED, &hotspots),
        [
            "staged",
            r#"#[kaa::action("Prepare the first value.")]"#,
            // Each stage's entry, and the transition that selects it.
            first,
            first,
            second,
            second,
            "return finish",
        ]
    );
}

#[test]
fn a_transition_a_stage_selects_again_draws_that_stage() {
    let source = include_str!("../../kaalang/tests/flow/stage/mutable_self_output.rs");
    let (_, hotspots) = draw(source, 0, false).unwrap();
    let code = code(source, &hotspots);
    let stage = r#"#[kaa::stage("Increment until two.")]"#;
    // Its entry and its transition to itself.
    assert_eq!(
        code.iter().filter(|code| **code == stage).count(),
        2,
        "{code:?}"
    );
}

#[test]
fn a_case_of_an_unwritten_choice_draws_the_choice() {
    let source = include_str!(
        "../../kaalang/tests/flow/choice/run_choice_skeleton_with_a_shared_consumer.rs"
    );
    let (_, hotspots) = draw(source, 0, false).unwrap();
    let choice = &code(source, &hotspots)
        .into_iter()
        .filter(|code| code.starts_with("#[kaa::choice("))
        .collect::<Vec<_>>();
    // The choice's select node, and each of its cases.
    assert!(choice.len() > 2, "{choice:?}");
    assert!(choice.iter().all(|code| *code == choice[0]), "{choice:?}");
}

#[test]
fn ranges_count_the_byte_order_mark_and_the_shebang() {
    let prefix = "\u{feff}#!/usr/bin/env rustx\n";
    let prefixed = format!("{prefix}{SOURCE}");
    let (svg, hotspots) = draw(SOURCE, 0, false).unwrap();
    let (prefixed_svg, prefixed_hotspots) = draw(&prefixed, 0, false).unwrap();
    assert_eq!(svg, prefixed_svg);
    assert_eq!(hotspots.len(), prefixed_hotspots.len());
    let spanned = |hotspot: &&Located| !matches!(hotspot.kind, Kind::Connection | Kind::Merge);
    for (hotspot, prefixed_hotspot) in hotspots
        .iter()
        .filter(spanned)
        .zip(prefixed_hotspots.iter().filter(spanned))
    {
        assert_eq!(
            prefixed_hotspot.source,
            hotspot.source.start + prefix.len()..hotspot.source.end + prefix.len()
        );
    }
}

/// A flow whose second block takes the first's wire and the flow's.
const CHAIN: &str = r#"#[kaa]
fn chain(first: u8) -> u8 {
    #[kaa::action("Double it.")]
    let second = |first| first * 2;

    #[kaa::action("Add them.")]
    let third = |first, second| first + second;

    |third| return third;
}
"#;

/// The ribbons of a diagram's connections and merges, each once, as names.
fn ribbons(source: &str) -> Vec<Vec<String>> {
    let (_, hotspots) = draw(source, 0, false).expect("the flow draws");
    let mut ribbons: Vec<Vec<String>> = hotspots
        .into_iter()
        .filter(|hotspot| matches!(hotspot.kind, Kind::Connection | Kind::Merge))
        .map(|hotspot| hotspot.ribbon.into_iter().map(|(name, _)| name).collect())
        .collect();
    ribbons.sort();
    ribbons.dedup();
    ribbons
}

#[test]
fn a_connection_lists_its_ribbon() {
    assert_eq!(
        ribbons(CHAIN),
        [vec!["first"], vec!["first", "second"], vec!["third"]]
    );
}

#[test]
fn a_merge_lists_its_ribbon() {
    let source = r#"#[kaa]
fn pick(condition: bool, first: u8) -> u8 {
    #[kaa::question("Take the yes branch?")]
    let (no, yes) = |condition| condition;

    #[kaa::action("Keep it.")]
    let value = |no, first| first;

    #[kaa::action("Raise it.")]
    let value = |yes, first| first + 1;

    |value| return value;
}
"#;
    let (svg, hotspots) = draw(source, 0, false).unwrap();
    let merge = drawn(svg.as_str()).merges[0];
    let at_merge = hotspots
        .iter()
        .find(|hotspot| {
            hotspot.kind == Kind::Merge
                && (
                    i32::midpoint(hotspot.left, hotspot.right),
                    i32::midpoint(hotspot.top, hotspot.bottom),
                ) == merge
        })
        .expect("the merge has a hotspot");
    let names: Vec<&str> = at_merge
        .ribbon
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(names, ["value"]);
}
