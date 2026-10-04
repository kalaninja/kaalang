//! Parses, measures, and wraps the restricted Markdown used by descriptions.

use std::{ops::Range, rc::Rc, sync::OnceLock};

use latex_rust::{
    BoxContent, Color as MathColor, Dim, MathBox, MathFont, MathStyle, SvgOptions, layout, parse,
    render_svg,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use unicode_segmentation::UnicodeSegmentation;

/// Larger formulas stay literal to keep diagram labels readable.
const MAX_FORMULA_HEIGHT_EM: i64 = 12;

// Rational em scales shared with the SVG text styles and formula decorations.
pub(crate) const SCRIPT_FONT_SCALE: (i32, i32) = (3, 4);
pub(crate) const SUPERSCRIPT_SHIFT: (i32, i32) = (2, 5);
pub(crate) const SUBSCRIPT_SHIFT: (i32, i32) = (1, 5);
const FORMULA_SLANT: (i32, i32) = (3, 20);
pub(crate) const FORMULA_UNDERLINE_OFFSET: (i32, i32) = (1, 10);
pub(crate) const FORMULA_STRIKETHROUGH_OFFSET: (i32, i32) = (1, 3);
pub(crate) const FORMULA_UNDERLINE_PADDING: i32 = 2;
pub(crate) const TEXT_ADVANCE_SCALE: i64 = 10_000;
const COLOR_MARKER_START: u32 = 0x0001_0203;

// Estimated character widths, in hundredths of an em.
const MONOSPACE_ADVANCE: i32 = 62;
const TAB_ADVANCE: i32 = 200;
const NARROW_ADVANCE: i32 = 32;
const WIDE_ADVANCE: i32 = 90;
const UPPERCASE_ADVANCE: i32 = 68;
const ASCII_ADVANCE: i32 = 56;
const ALPHABETIC_ADVANCE: i32 = 60;
const FULL_WIDTH_ADVANCE: i32 = 100;
const CYRILLIC_BLOCK_END: u32 = 0x0500;
const EMPHASIS_WIDTH_PERCENT: i64 = 105;
const TEXT_ASCENT_PARTS: i32 = 4;
const TEXT_EM_PARTS: i32 = 5;
const SUPERSCRIPT_ASCENT_PERCENT: i32 = 90;
const SUBSCRIPT_DESCENT_PERCENT: i32 = 30;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // A span combines independent CSS effects.
pub(crate) struct Style {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) strikethrough: bool,
    pub(crate) underline: bool,
    pub(crate) highlight: bool,
    pub(crate) code: bool,
    pub(crate) quote: bool,
    pub(crate) script: Option<Script>,
    pub(crate) color: Option<Color>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Color {
    Red,
    Green,
    Blue,
    Purple,
    Muted,
}

impl Color {
    pub(crate) const fn class(self) -> &'static str {
        match self {
            Self::Red => "md-color-red",
            Self::Green => "md-color-green",
            Self::Blue => "md-color-blue",
            Self::Purple => "md-color-purple",
            Self::Muted => "md-color-muted",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Script {
    Superscript,
    Subscript,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A TeX formula laid out once and ready to scale into an SVG label.
pub(crate) struct Formula {
    layout: MathBox,
    svg_body: String,
    clip_width: Option<i32>,
}

impl Formula {
    fn parse(source: &str, display: bool) -> Option<Self> {
        let font = math_font()?;
        let ast = parse(source).ok()?;
        let style = if display {
            MathStyle::Display
        } else {
            MathStyle::Text
        };
        let layout = layout(&ast, font, style).ok()?;
        if layout.width <= Dim::zero() || &layout.height + &layout.depth < Dim::zero() {
            return None;
        }
        if layout.height.abs() + layout.depth.abs() > Dim::from_i64(MAX_FORMULA_HEIGHT_EM) {
            return None;
        }
        let mut options = SvgOptions::new();
        options.font_size_pt = Dim::one();
        // The backend writes its default color into every rule. Pick a color
        // absent from the formula so only that default becomes the label color.
        let mut marker = COLOR_MARKER_START;
        while uses_color(&layout, marker_color(marker)) {
            marker += 1;
        }
        options.color = marker_color(marker);
        let document = render_svg(&layout, font, &options).ok()?;
        // Invalid dimensions can survive layout, including inside nested boxes.
        if document.contains("NaN") {
            return None;
        }
        let svg = document.find("<svg ")?;
        let body = svg + document[svg..].find('>')? + 1;
        let end = document.rfind("</svg>")?;
        let hex = options.color.css_hex();
        let svg_body = document[body..end]
            .replace(&format!("fill=\"{hex}\""), "fill=\"inherit\"")
            .replace(&format!("stroke=\"{hex}\""), "stroke=\"currentColor\"");
        Some(Self {
            layout,
            svg_body,
            clip_width: None,
        })
    }

    fn size(font_size: i32, style: Style) -> Dim {
        let script = if style.script.is_some() {
            Dim::ratio(SCRIPT_FONT_SCALE.0.into(), SCRIPT_FONT_SCALE.1.into())
        } else {
            Dim::one()
        };
        script * Dim::from_i64(i64::from(font_size))
    }

    pub(crate) fn slant(&self, style: Style) -> Dim {
        if style.italic || style.quote {
            &self.view_box_height() * Dim::ratio(FORMULA_SLANT.0.into(), FORMULA_SLANT.1.into())
        } else {
            Dim::zero()
        }
    }

    pub(crate) fn width(&self, font_size: i32, style: Style) -> Dim {
        let width = (&self.layout.width + self.slant(style)) * Self::size(font_size, style);
        self.clip_width.map_or_else(
            || width.clone(),
            |limit| width.min(&Dim::from_i64(i64::from(limit))),
        )
    }

    pub(crate) fn visible_width(&self, font_size: i32, style: Style) -> Dim {
        let width = self.width(font_size, style);
        if self.is_clipped() {
            (&width - Dim::from_i64(i64::from(font_size))).max(&Dim::zero())
        } else {
            width
        }
    }

    pub(crate) fn ascent(&self, font_size: i32, style: Style) -> Dim {
        &self.layout.height * Self::size(font_size, style) + Self::baseline_shift(font_size, style)
    }

    pub(crate) fn descent(&self, font_size: i32, style: Style) -> Dim {
        &self.layout.depth * Self::size(font_size, style) - Self::baseline_shift(font_size, style)
    }

    fn baseline_shift(font_size: i32, style: Style) -> Dim {
        let (numerator, denominator) = match style.script {
            Some(Script::Superscript) => SUPERSCRIPT_SHIFT,
            Some(Script::Subscript) => (-SUBSCRIPT_SHIFT.0, SUBSCRIPT_SHIFT.1),
            None => (0, 1),
        };
        Dim::ratio(
            i64::from(font_size) * i64::from(numerator),
            denominator.into(),
        )
    }

    pub(crate) fn view_box(
        &self,
        font_size: i32,
        style: Style,
        top_padding: i32,
        bottom_padding: i32,
    ) -> String {
        let size = Self::size(font_size, style);
        format!(
            "0 {} {} {}",
            svg_dimension(&(-Dim::from_i64(i64::from(top_padding)) / &size)),
            svg_dimension(&(self.visible_width(font_size, style) / &size)),
            svg_dimension(
                &(self.view_box_height()
                    + Dim::from_i64(i64::from(top_padding + bottom_padding)) / size)
            )
        )
    }

    pub(crate) fn is_clipped(&self) -> bool {
        self.clip_width.is_some()
    }

    pub(crate) fn view_box_width(&self, style: Style) -> Dim {
        &self.layout.width + self.slant(style)
    }

    pub(crate) fn view_box_height(&self) -> Dim {
        &self.layout.height + &self.layout.depth
    }

    pub(crate) fn svg_body(&self) -> &str {
        &self.svg_body
    }
}

fn marker_color(marker: u32) -> MathColor {
    let [_, red, green, blue] = marker.to_be_bytes();
    MathColor::rgb(red, green, blue)
}

fn uses_color(layout: &MathBox, color: MathColor) -> bool {
    match &layout.content {
        BoxContent::Color(value, inner) | BoxContent::BackColor(value, inner) => {
            *value == color || uses_color(inner, color)
        }
        BoxContent::Frame { stroke, inner, .. } => {
            stroke.is_some_and(|value| value == color) || uses_color(inner, color)
        }
        BoxContent::HList(children)
        | BoxContent::VList(children)
        | BoxContent::Overlap(children) => children.iter().any(|child| uses_color(child, color)),
        _ => false,
    }
}

fn math_font() -> Option<&'static MathFont> {
    static FONT: OnceLock<Option<MathFont>> = OnceLock::new();
    FONT.get_or_init(|| MathFont::stix_two_math().ok()).as_ref()
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A contiguous piece of text whose characters share the same style.
pub(crate) struct StyledSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
    /// Shared: measurement clones spans per grapheme, and a laid-out formula
    /// carries its whole serialized body.
    pub(crate) formula: Option<Rc<Formula>>,
}

/// Parsed text spans plus their accessibility projection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RichText {
    spans: Vec<StyledSpan>,
    plain: String,
}

impl RichText {
    pub(crate) fn literal(text: impl Into<String>) -> Self {
        let mut result = Self::default();
        result.push(text.into(), Style::default());
        result.finish();
        result
    }

    pub(crate) fn markdown(text: &str) -> Self {
        Self::parse(text, true)
    }

    /// The same text without laying out its formulas, which display as their
    /// literal source instead. Label validation reads the decoded characters
    /// and never the math, so it skips the expensive half of parsing.
    pub(crate) fn markdown_text(text: &str) -> Self {
        Self::parse(text, false)
    }

    fn parse(text: &str, formulas: bool) -> Self {
        let mut result = Self::default();
        for (index, line) in text.split('\n').enumerate() {
            if index != 0 {
                result.push("\n", Style::default());
            }
            parse_line(line, formulas, &mut result);
        }
        result.finish();
        result
    }

    pub(crate) fn spans(&self) -> &[StyledSpan] {
        &self.spans
    }

    /// A formula is one cluster whose text is its whole source, so it goes
    /// in one piece.
    pub(crate) fn pop_grapheme(&mut self) {
        let Some(last) = clusters(self).pop() else {
            return;
        };
        let mut remaining = last.text.len();
        for span in self.spans.iter_mut().rev() {
            let removed = remaining.min(span.text.len());
            span.text.truncate(span.text.len() - removed);
            remaining -= removed;
            if remaining == 0 {
                break;
            }
        }
        self.spans.retain(|span| !span.text.is_empty());
        self.finish();
    }

    pub(crate) fn push_literal(&mut self, text: &str) {
        self.push(text, Style::default());
        self.finish();
    }

    fn push(&mut self, text: impl AsRef<str>, style: Style) {
        let text = text.as_ref();
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.style == style
            && last.formula.is_none()
        {
            last.text.push_str(text);
        } else {
            self.spans.push(StyledSpan {
                text: text.to_owned(),
                style,
                formula: None,
            });
        }
    }

    fn push_span(&mut self, span: StyledSpan) {
        if span.formula.is_none() {
            self.push(span.text, span.style);
        } else {
            self.spans.push(span);
        }
    }

    fn push_formula(&mut self, source: &str, display: bool, style: Style) -> bool {
        let Some(formula) = Formula::parse(source, display) else {
            return false;
        };
        self.spans.push(StyledSpan {
            text: source.to_owned(),
            style,
            formula: Some(Rc::new(formula)),
        });
        true
    }

    fn finish(&mut self) {
        self.plain.clear();
        let mut script = None;
        for span in &self.spans {
            if span.style.script != script {
                if script.is_some() {
                    self.plain.push(')');
                }
                match span.style.script {
                    Some(Script::Superscript) => self.plain.push_str("^("),
                    Some(Script::Subscript) => self.plain.push_str("_("),
                    None => {}
                }
                script = span.style.script;
            }
            self.plain.push_str(&span.text);
        }
        if script.is_some() {
            self.plain.push(')');
        }
    }
}

impl AsRef<str> for RichText {
    fn as_ref(&self) -> &str {
        &self.plain
    }
}

#[derive(Clone, Copy)]
enum InlineEffect {
    Underline,
    Highlight,
    Color(Color),
}

impl InlineEffect {
    const fn tag(self) -> &'static str {
        match self {
            Self::Underline => "u",
            Self::Highlight => "mark",
            Self::Color(_) => "color",
        }
    }
}

enum InlineTag {
    Open(InlineEffect),
    Close(&'static str),
}

fn inline_tag(source: &str) -> Option<InlineTag> {
    Some(match source {
        "<u>" => InlineTag::Open(InlineEffect::Underline),
        "</u>" => InlineTag::Close("u"),
        "<mark>" => InlineTag::Open(InlineEffect::Highlight),
        "</mark>" => InlineTag::Close("mark"),
        "</color>" => InlineTag::Close("color"),
        _ => {
            let name = source.strip_prefix("<color name=\"")?.strip_suffix("\">")?;
            let color = match name {
                "red" => Color::Red,
                "green" => Color::Green,
                "blue" => Color::Blue,
                "purple" => Color::Purple,
                "muted" => Color::Muted,
                _ => return None,
            };
            InlineTag::Open(InlineEffect::Color(color))
        }
    })
}

/// Only balanced, properly nested exact tags affect the parsed line. A tag
/// inside a construct the line copies literally, such as a link, is part of
/// that literal text and pairs with nothing outside it.
fn paired_tags(events: &[(Event<'_>, Range<usize>)]) -> Vec<bool> {
    let mut paired = vec![false; events.len()];
    let mut opened = Vec::new();
    let mut literal_depth = 0_usize;
    let literal =
        |tag: TagEnd| !is_effect(tag) && !matches!(tag, TagEnd::Paragraph | TagEnd::BlockQuote(_));
    for (current, (event, _)) in events.iter().enumerate() {
        match event {
            Event::Start(tag) if literal(tag.to_end()) => literal_depth += 1,
            Event::End(tag) if literal(*tag) => literal_depth = literal_depth.saturating_sub(1),
            Event::InlineHtml(source) if literal_depth == 0 => match inline_tag(source) {
                Some(InlineTag::Open(effect)) => opened.push((current, effect)),
                Some(InlineTag::Close(tag))
                    if opened.last().is_some_and(|(_, effect)| effect.tag() == tag) =>
                {
                    let (start, _) = opened.pop().expect("a matching tag was just found");
                    if events[start].1.end != events[current].1.start {
                        paired[start] = true;
                        paired[current] = true;
                    }
                }
                Some(InlineTag::Close(tag)) => {
                    if let Some(position) =
                        opened.iter().rposition(|(_, effect)| effect.tag() == tag)
                    {
                        opened.truncate(position);
                    }
                }
                None => {}
            },
            _ => {}
        }
    }
    paired
}

const fn effect_marker_len(effect: TagEnd) -> usize {
    match effect {
        TagEnd::Strong | TagEnd::Strikethrough => 2,
        _ => 1,
    }
}

#[allow(clippy::too_many_lines)] // One pass keeps parser events and their source ranges together.
fn parse_line(line: &str, formulas: bool, output: &mut RichText) {
    if line.is_empty() {
        return;
    }
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_MATH;
    let events = Parser::new_ext(line, options)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let Some((mut index, end, paragraph, quoted)) = paragraph(&events) else {
        output.push(line, Style::default());
        return;
    };
    let paired = paired_tags(&events);

    if quoted {
        let marker = line[..paragraph.start]
            .find('>')
            .expect("a quoted paragraph has a marker");
        output.push(&line[..marker], Style::default());
        output.push("“", active_style(&[], &[], true));
        // The marker is `>` and at most one space; anything further is content.
        let after = marker + 1;
        let content = after + usize::from(line[after..].starts_with([' ', '\t']));
        output.push(
            &line[content.min(paragraph.start)..paragraph.start],
            active_style(&[], &[], true),
        );
    } else {
        output.push(&line[..paragraph.start], Style::default());
    }
    let mut effects = Vec::new();
    let mut inline_effects = Vec::new();
    let mut consumed = paragraph.start;
    let mut pending = InlineText::default();
    while index < end {
        let (event, range) = &events[index];
        // The style the events before this one left active. No arm below reads
        // it after changing `effects` or `inline_effects`.
        let style = active_style(&effects, &inline_effects, quoted);
        if !matches!(event, Event::Text(_)) {
            pending.flush(output, style);
        }
        match event {
            Event::Start(tag) if is_effect(tag.to_end()) => {
                effects.push(tag.to_end());
                consumed = consumed.max(range.start + effect_marker_len(tag.to_end()));
            }
            // The parser nests its events, and the arm below skips whole
            // literal subtrees, so an effect always ends the innermost one.
            Event::End(tag) if is_effect(*tag) => {
                effects.pop();
                consumed = consumed.max(range.end);
            }
            Event::Start(tag) => {
                output.push(&line[range.clone()], style);
                consumed = consumed.max(range.end);
                let end = tag.to_end();
                if let Some(offset) = events[index + 1..].iter().position(|(event, candidate)| {
                    matches!(event, Event::End(candidate_end) if *candidate_end == end)
                        && candidate == range
                }) {
                    index += offset + 1;
                }
            }
            Event::Text(text) => {
                pending.push(text, line, range.clone());
                consumed = consumed.max(range.end);
            }
            Event::Code(text) => {
                let mut code = style;
                code.code = true;
                output.push(text, code);
                consumed = consumed.max(range.end);
            }
            Event::InlineMath(source) | Event::DisplayMath(source) => {
                let display = matches!(event, Event::DisplayMath(_));
                if !formulas || !output.push_formula(source, display, style) {
                    output.push(&line[range.clone()], style);
                }
                consumed = consumed.max(range.end);
            }
            Event::InlineHtml(source) if paired[index] => {
                match inline_tag(source).expect("a paired tag is recognized") {
                    InlineTag::Open(effect) => inline_effects.push(effect),
                    // `paired_tags` pairs a close only with the innermost open.
                    InlineTag::Close(_) => {
                        inline_effects.pop();
                    }
                }
                consumed = consumed.max(range.end);
            }
            Event::InlineHtml(_) | Event::Html(_) => {
                output.push(&line[range.clone()], style);
                consumed = consumed.max(range.end);
            }
            Event::End(TagEnd::Paragraph) => {}
            _ => {
                if range.end > consumed {
                    output.push(&line[consumed..range.end], style);
                    consumed = range.end;
                }
            }
        }
        index += 1;
    }
    pending.flush(output, active_style(&effects, &inline_effects, quoted));
    output.push(&line[consumed..], active_style(&[], &[], quoted));
    if quoted {
        output.push("”", active_style(&[], &[], true));
    }
}

fn paragraph(events: &[(Event<'_>, Range<usize>)]) -> Option<(usize, usize, Range<usize>, bool)> {
    match events {
        [
            (Event::Start(Tag::Paragraph), range),
            ..,
            (Event::End(TagEnd::Paragraph), _),
        ] => Some((1, events.len() - 1, range.clone(), false)),
        [
            (Event::Start(Tag::BlockQuote(None)), _),
            (Event::Start(Tag::Paragraph), range),
            ..,
            (Event::End(TagEnd::Paragraph), _),
            (Event::End(TagEnd::BlockQuote(None)), _),
        ] => Some((2, events.len() - 2, range.clone(), true)),
        _ => None,
    }
}

/// Adjacent parser text events, retaining which decoded markers were literal.
#[derive(Default)]
struct InlineText {
    text: String,
    literal_markers: Vec<usize>,
}

impl InlineText {
    fn push(&mut self, text: &str, source: &str, range: Range<usize>) {
        let entity = text != &source[range.clone()];
        let escaped = source[..range.start]
            .bytes()
            .rev()
            .take_while(|b| *b == b'\\')
            .count()
            % 2
            == 1;
        self.literal_markers
            .extend(text.char_indices().filter_map(|(index, ch)| {
                (matches!(ch, '^' | '~') && (entity || (escaped && index == 0)))
                    .then_some(self.text.len() + index)
            }));
        self.text.push_str(text);
    }

    /// Add only the intraword exception; other delimiter matching belongs to
    /// pulldown-cmark. Code, formulas and literal fallback never enter this buffer.
    fn flush(&mut self, output: &mut RichText, base_style: Style) {
        let mut rest = self.text.as_str();
        while let Some((open, marker)) = rest.char_indices().find(|(index, character)| {
            matches!(character, '^' | '~')
                && self.is_marker(self.text.len() - rest.len() + index, *character)
        }) {
            let after_open = open + marker.len_utf8();
            // Only a nonempty, unpadded run touching a word is a script.
            let Some(close) = rest[after_open..]
                .char_indices()
                .find(|(index, character)| {
                    *character == marker
                        && self.is_marker(
                            self.text.len() - rest.len() + after_open + index,
                            *character,
                        )
                })
                .map(|(index, _)| after_open + index)
                .filter(|&close| {
                    let inside = &rest[after_open..close];
                    let previous = rest[..open].chars().next_back();
                    let following = rest[close + marker.len_utf8()..].chars().next();
                    !inside.is_empty()
                        && !inside.starts_with(char::is_whitespace)
                        && !inside.ends_with(char::is_whitespace)
                        && (previous.is_some_and(char::is_alphanumeric)
                            || following.is_some_and(char::is_alphanumeric))
                })
            else {
                output.push(&rest[..after_open], base_style);
                rest = &rest[after_open..];
                continue;
            };
            let inside = &rest[after_open..close];
            output.push(&rest[..open], base_style);
            let mut style = base_style;
            style.script = Some(if marker == '^' {
                Script::Superscript
            } else {
                Script::Subscript
            });
            output.push(inside, style);
            rest = &rest[close + marker.len_utf8()..];
        }
        output.push(rest, base_style);
        self.text.clear();
        self.literal_markers.clear();
    }

    /// A lone index marker: not itself literal, and not half of a pair the
    /// parser already owns. The neighbour tests short-circuit at index 0, where
    /// the preceding text is empty and `index - 1` has nothing to name.
    fn is_marker(&self, index: usize, marker: char) -> bool {
        let literal = |at| self.literal_markers.binary_search(&at).is_ok();
        !literal(index)
            && (!self.text[..index].ends_with(marker) || literal(index - 1))
            && (!self.text[index + 1..].starts_with(marker) || literal(index + 1))
    }
}

const fn is_effect(tag: TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Strong
            | TagEnd::Emphasis
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
    )
}

fn active_style(effects: &[TagEnd], inline: &[InlineEffect], quote: bool) -> Style {
    Style {
        bold: effects.contains(&TagEnd::Strong),
        italic: effects.contains(&TagEnd::Emphasis),
        strikethrough: effects.contains(&TagEnd::Strikethrough),
        underline: inline
            .iter()
            .any(|effect| matches!(effect, InlineEffect::Underline)),
        highlight: inline
            .iter()
            .any(|effect| matches!(effect, InlineEffect::Highlight)),
        code: false,
        quote,
        script: effects.iter().rev().find_map(|effect| match effect {
            TagEnd::Superscript => Some(Script::Superscript),
            TagEnd::Subscript => Some(Script::Subscript),
            _ => None,
        }),
        color: inline.iter().rev().find_map(|effect| match effect {
            InlineEffect::Color(color) => Some(*color),
            _ => None,
        }),
    }
}

/// Approximate advance width of one character, in hundredths of an em, for the
/// sans-serif stack the stylesheet requests. The renderer has no font metrics,
/// so this estimate errs wide rather than letting a label leave its node.
fn advance(character: char, code: bool) -> i32 {
    if code && character.is_ascii() && character != '\t' {
        return MONOSPACE_ADVANCE;
    }
    match character {
        '\t' => TAB_ADVANCE,
        ' ' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '`' | 'i' | 'j' | 'l' | 'I' | '('
        | ')' | '[' | ']' | '{' | '}' | '/' | '\\' | '-' => NARROW_ADVANCE,
        'm' | 'w' | 'M' | 'W' | '@' => WIDE_ADVANCE,
        'A'..='Z' => UPPERCASE_ADVANCE,
        _ if character.is_ascii() => ASCII_ADVANCE,
        // Latin-1, Greek, and Cyrillic behave like Latin; assume anything
        // beyond them, such as CJK, is full width.
        _ if (character as u32) < CYRILLIC_BLOCK_END => ALPHABETIC_ADVANCE,
        _ => FULL_WIDTH_ADVANCE,
    }
}

fn cluster_advance(cluster: &str, style: Style, font_size: i32) -> i64 {
    let character = cluster
        .chars()
        .next()
        .expect("a grapheme cluster has at least one character");
    let scale = if style.script.is_some() {
        SCRIPT_FONT_SCALE.0 * 100 / SCRIPT_FONT_SCALE.1
    } else {
        100
    };
    let emphasis = if style.bold || style.italic || style.quote {
        EMPHASIS_WIDTH_PERCENT
    } else {
        100
    };
    i64::from(advance(character, style.code)) * i64::from(font_size) * emphasis * i64::from(scale)
        / 100
}

/// One dimension as an SVG attribute value, rounded to four fractional digits.
/// `Dim` is a rational, so its exact expansion can run to 24 digits, which only
/// makes the reviewed diagrams harder to read. A value too small to survive that
/// rounding keeps its exact expansion instead: a zero viewBox side would blank
/// the formula it measures.
pub(crate) fn svg_dimension(dimension: &Dim) -> String {
    const DIGITS: usize = 4;
    const SCALE: i128 = 10_000;

    let exact = dimension.to_svg_string();
    let Some((whole, fraction)) = exact.split_once('.') else {
        return exact;
    };
    let negative = whole.starts_with('-');
    let Ok(units) = whole.trim_start_matches('-').parse::<i128>() else {
        return exact;
    };
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return exact;
    }
    let mut digits = fraction.bytes().map(|byte| i128::from(byte - b'0'));
    let kept = (0..DIGITS).fold(0, |kept, _| kept * 10 + digits.next().unwrap_or(0));
    let Some(mut value) = units
        .checked_mul(SCALE)
        .and_then(|units| units.checked_add(kept))
    else {
        return exact;
    };
    // Round half away from zero on the first digit this drops.
    if digits.next().is_some_and(|digit| digit >= 5) {
        value += 1;
    }
    if value == 0 {
        return if dimension.is_zero() {
            "0".to_owned()
        } else {
            exact
        };
    }
    let text = format!("{}.{:0>DIGITS$}", value / SCALE, value % SCALE)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned();
    if negative { format!("-{text}") } else { text }
}

fn dimension_ceiling(dimension: &Dim) -> i32 {
    dimension
        .max(&Dim::zero())
        .ceil_to_u32()
        .ok()
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(i32::MAX)
}

/// The plain text of a wrapped block, for a diagnostic that names a label.
pub(crate) fn joined(lines: &[RichText], separator: &str) -> String {
    lines
        .iter()
        .map(RichText::as_ref)
        .collect::<Vec<_>>()
        .join(separator)
}

pub(crate) fn text_width(text: &RichText, font_size: i32) -> i32 {
    clusters_width(&clusters(text), font_size)
}

/// The first style and advance own a grapheme that crosses span boundaries.
/// A formula span is a single cluster, so it never joins its neighbors.
pub(crate) fn shaping_spans(text: &RichText, font_size: i32) -> Vec<(StyledSpan, i64, usize)> {
    clusters(text)
        .chunk_by(|left, right| left.first_span == right.first_span)
        .map(|run| {
            let mut first = run[0].spans[0].clone();
            if first.formula.is_none() {
                first.text = run.iter().map(|cluster| cluster.text.as_str()).collect();
            }
            let advance = run.iter().map(|cluster| cluster.advance(font_size)).sum();
            (first, advance, run[0].first_span)
        })
        .collect()
}

/// One Unicode grapheme (possibly spanning styles), or one indivisible formula.
struct Cluster {
    text: String,
    spans: Vec<StyledSpan>,
    first_span: usize,
}

impl Cluster {
    fn advance(&self, font_size: i32) -> i64 {
        if self.text.ends_with('\n') {
            return 0;
        }
        self.spans[0].formula.as_ref().map_or_else(
            || cluster_advance(&self.text, self.spans[0].style, font_size),
            |formula| {
                i64::from(dimension_ceiling(
                    &formula.width(font_size, self.spans[0].style),
                )) * TEXT_ADVANCE_SCALE
            },
        )
    }
}

fn clusters(text: &RichText) -> Vec<Cluster> {
    // NUL is a grapheme boundary on both sides, so math stays atomic.
    let visible: String = text
        .spans
        .iter()
        .map(|span| {
            if span.formula.is_some() {
                "\0"
            } else {
                span.text.as_str()
            }
        })
        .collect();
    let mut index = 0;
    let mut offset = 0;
    visible
        .graphemes(true)
        .map(|grapheme| {
            let mut remaining = grapheme.len();
            let mut cluster = Cluster {
                text: String::new(),
                spans: Vec::new(),
                first_span: index,
            };
            while remaining > 0 {
                let span = &text.spans[index];
                // A formula's NUL stands for its whole source.
                let (taken, part) = if span.formula.is_some() {
                    (1, span.text.as_str())
                } else {
                    let taken = remaining.min(span.text.len() - offset);
                    (taken, &span.text[offset..offset + taken])
                };
                cluster.text.push_str(part);
                cluster.spans.push(StyledSpan {
                    text: part.to_owned(),
                    style: span.style,
                    formula: span.formula.clone(),
                });
                remaining -= taken;
                offset += taken;
                if span.formula.is_some() || offset == span.text.len() {
                    index += 1;
                    offset = 0;
                }
            }
            cluster
        })
        .collect()
}

fn clusters_width(clusters: &[Cluster], font_size: i32) -> i32 {
    clusters
        .iter()
        .map(|cluster| cluster.advance(font_size))
        .sum::<i64>()
        .checked_div(TEXT_ADVANCE_SCALE)
        .and_then(|width| i32::try_from(width).ok())
        .unwrap_or(i32::MAX)
}

fn fitting_count(clusters: &[Cluster], budget: i32, font_size: i32) -> usize {
    let mut used = 0;
    for (count, cluster) in clusters.iter().enumerate() {
        used += cluster.advance(font_size);
        if used / TEXT_ADVANCE_SCALE > i64::from(budget) {
            return count.max(1);
        }
    }
    clusters.len()
}

/// Wraps at the last fitting space, or between grapheme clusters for long words.
/// Preserves text except newline separators; an oversized cluster stays intact.
pub(crate) fn wrap_text(text: &RichText, budget: i32, font_size: i32) -> Vec<RichText> {
    let mut all = clusters(text);
    for cluster in &mut all {
        if cluster.advance(font_size) > i64::from(budget) * TEXT_ADVANCE_SCALE
            && let Some(formula) = &mut cluster.spans[0].formula
        {
            Rc::make_mut(formula).clip_width = Some(budget.max(1));
        }
    }

    let mut lines = Vec::new();
    // A CRLF is one grapheme cluster, so match the terminator rather than the
    // whole cluster; either way the separator leaves no ink.
    for clusters in all.split(|cluster| cluster.text.ends_with('\n')) {
        if clusters.is_empty() {
            lines.push(RichText::default());
            continue;
        }
        let mut start = 0;
        while clusters_width(&clusters[start..], font_size) > budget {
            let hard_end = start + fitting_count(&clusters[start..], budget, font_size);
            let preferred_break = clusters[start..hard_end]
                .iter()
                .rposition(|cluster| cluster.text.starts_with(char::is_whitespace))
                .map(|offset| start + offset + 1)
                .filter(|end| *end > start + 1);
            let end = preferred_break.unwrap_or(hard_end);
            lines.push(line(&clusters[start..end]));
            start = end;
        }
        if start < clusters.len() {
            lines.push(line(&clusters[start..]));
        }
    }
    lines
}

pub(crate) fn wrap_literal(text: &str, budget: i32, font_size: i32) -> Vec<RichText> {
    wrap_text(&RichText::literal(text), budget, font_size)
}

fn line(clusters: &[Cluster]) -> RichText {
    let mut line = RichText::default();
    for cluster in clusters {
        for span in &cluster.spans {
            line.push_span(span.clone());
        }
    }
    line.finish();
    line
}

/// Height and baseline positions for a wrapped block of label text.
pub(crate) struct TextBlockMetrics {
    pub(crate) height: i32,
    pub(crate) baselines: Vec<i32>,
}

pub(crate) fn line_ink(spans: &[StyledSpan], font_size: i32) -> (i32, i32) {
    let mut ascent = (font_size * TEXT_ASCENT_PARTS + (TEXT_EM_PARTS - 1)) / TEXT_EM_PARTS;
    let mut descent = (font_size + (TEXT_EM_PARTS - 1)) / TEXT_EM_PARTS;
    for span in spans {
        if let Some(formula) = &span.formula {
            ascent = ascent.max(dimension_ceiling(&formula.ascent(font_size, span.style)));
            let padding = i32::from(span.style.underline) * FORMULA_UNDERLINE_PADDING;
            let below = dimension_ceiling(&formula.descent(font_size, span.style));
            descent = descent.max(below.saturating_add(padding));
        }
        // Match the script font scale and baseline shifts in the SVG stylesheet.
        match span.style.script {
            Some(Script::Superscript) => {
                ascent = ascent.max((font_size * SUPERSCRIPT_ASCENT_PERCENT + 99) / 100);
            }
            Some(Script::Subscript) => {
                descent = descent.max((font_size * SUBSCRIPT_DESCENT_PERCENT + 99) / 100);
            }
            None => {}
        }
    }
    (ascent, descent)
}

pub(crate) fn block_metrics(
    lines: &[RichText],
    font_size: i32,
    line_height: i32,
) -> TextBlockMetrics {
    let (ascent, descent) = line_ink(&[], font_size);
    let leading = (line_height - ascent - descent).max(0);
    let mut height = 0;
    let mut baselines = Vec::with_capacity(lines.len());
    for line in lines {
        let (ascent, descent) = line_ink(line.spans(), font_size);
        baselines.push(height + leading + ascent);
        height += line_height.max(leading + ascent + descent);
    }
    TextBlockMetrics { height, baselines }
}

#[cfg(test)]
mod tests {
    use super::{
        Color, Dim, RichText, Script, Style, joined, line_ink, svg_dimension, text_width,
        wrap_literal, wrap_text,
    };
    use crate::layout::{LABEL_FONT, NODE_LABEL_WIDTH};

    #[test]
    fn indices_do_not_reinterpret_code_escapes_entities_or_literal_fallbacks() {
        for (source, expected) in [
            ("`x^2^ H~2~O`", "x^2^ H~2~O"),
            (r"[x^2^](url)", r"[x^2^](url)"),
            (r"$\unknown{x^2^}$", r"$\unknown{x^2^}$"),
            (r"x\\^2^", r"x\^(2)"),
            (r"x\^2\^", "x^2^"),
            ("x&#94;2&#94;", "x^2^"),
            (r"x\*a^2^", "x*a^(2)"),
            (r"x^a\^b^", "x^(a^b)"),
            (r"$x\^2$", r"$x\^2$"),
            ("A stray ^ and H~2~O", "A stray ^ and H_(2)O"),
        ] {
            assert_eq!(RichText::markdown(source).as_ref(), expected, "{source}");
        }
        assert_eq!(RichText::markdown("&#30;&#31;").as_ref(), "\u{1e}\u{1f}");
    }

    #[test]
    fn parses_the_supported_inline_effects_and_literal_fallbacks() {
        let text = RichText::markdown(
            "  **bold** *italic* ~~old~~ `code` ^super^ ~sub~ $x^2$ [**link**](url) <b>raw</b>  ",
        );

        assert_eq!(
            text.as_ref(),
            "  bold italic old code ^(super) _(sub) x^2 [**link**](url) <b>raw</b>  "
        );
        assert!(text.spans().iter().any(|span| span.style.bold));
        assert!(text.spans().iter().any(|span| span.style.italic));
        assert!(text.spans().iter().any(|span| span.style.strikethrough));
        assert!(text.spans().iter().any(|span| span.style.code));
        assert!(
            text.spans()
                .iter()
                .any(|span| span.style.script == Some(Script::Superscript))
        );
        assert!(
            text.spans()
                .iter()
                .any(|span| span.style.script == Some(Script::Subscript))
        );
        assert!(text.spans().iter().any(|span| span.formula.is_some()));
        let quote = RichText::markdown("> A **quoted** line.");
        assert_eq!(quote.as_ref(), "“A quoted line.”");
        assert_eq!(RichText::markdown(">   spaced").as_ref(), "“  spaced”");
        assert!(quote.spans().iter().all(|span| span.style.quote));
        assert_eq!(
            RichText::markdown(r"Bad $\unknown{}$ formula.").as_ref(),
            r"Bad $\unknown{}$ formula."
        );
        assert_eq!(
            RichText::markdown(r"$\color{not-a-color}x$").as_ref(),
            r"$\color{not-a-color}x$"
        );
        assert_eq!(
            RichText::markdown("# **heading**").as_ref(),
            "# **heading**"
        );
        assert_eq!(
            RichText::markdown("x^2^ + H~2~O").as_ref(),
            "x^(2) + H_(2)O"
        );
        assert_eq!(RichText::markdown(r"x\^2\^ `x\^2`").as_ref(), r"x^2^ x\^2");
    }

    #[test]
    fn paired_inline_styles_preserve_content_and_literal_fallbacks() {
        let text = RichText::markdown(
            r#"<u>under</u> <mark>**bright**</mark> <color name="red">red <color name="blue">blue</color> red</color>"#,
        );
        assert_eq!(text.as_ref(), "under bright red blue red");
        assert!(
            text.spans()
                .iter()
                .any(|span| span.text == "under" && span.style.underline)
        );
        assert!(
            text.spans()
                .iter()
                .any(|span| span.text == "bright" && span.style.highlight && span.style.bold)
        );
        assert!(
            text.spans()
                .iter()
                .any(|span| span.text == "blue" && span.style.color == Some(Color::Blue))
        );
        assert_eq!(text.spans().last().unwrap().style.color, Some(Color::Red));
        for (source, expected) in [
            ("<u></u>", "<u></u>"),
            ("<mark></mark>", "<mark></mark>"),
            (
                "<color name=\"red\"></color>",
                "<color name=\"red\"></color>",
            ),
            ("a<mark></mark>b", "a<mark></mark>b"),
            ("<u>unclosed", "<u>unclosed"),
            ("<mark>unclosed", "<mark>unclosed"),
            ("<u><mark>x</u></mark>", "<u><mark>x</u></mark>"),
            (
                "<color name=\"orange\">other</color>",
                "<color name=\"orange\">other</color>",
            ),
            (
                "<color name=\"red\" class=\"x\">other</color>",
                "<color name=\"red\" class=\"x\">other</color>",
            ),
            ("`<u>code</u>`", "<u>code</u>"),
            (r"\<u>escaped</u>", "<u>escaped</u>"),
            ("<u>a [b</u>](url) c", "<u>a [b</u>](url) c"),
        ] {
            let parsed = RichText::markdown(source);
            assert_eq!(parsed.as_ref(), expected, "{source}");
            assert!(parsed.spans().iter().all(|span| !span.style.underline
                && !span.style.highlight
                && span.style.color.is_none()));
        }
        let mixed = RichText::markdown("<u>valid</u> <mark>unclosed");
        assert_eq!(mixed.as_ref(), "valid <mark>unclosed");
        assert!(mixed.spans()[0].style.underline);
    }

    #[test]
    fn unsupported_html_tags_allow_supported_formatting() {
        let source = "**outside** <b>**warning** <u>raw</u> <b>*nested*</b></b> <u>after</u>";
        let parsed = RichText::markdown(source);
        assert_eq!(
            parsed.as_ref(),
            "outside <b>warning raw <b>nested</b></b> after"
        );
        assert!(parsed.spans()[0].style.bold);
        assert!(
            parsed
                .spans()
                .iter()
                .any(|span| span.text == "warning" && span.style.bold)
        );
        assert!(
            parsed
                .spans()
                .iter()
                .any(|span| span.text == "raw" && span.style.underline)
        );
        assert!(
            parsed
                .spans()
                .iter()
                .any(|span| span.text == "nested" && span.style.italic)
        );
        assert!(parsed.spans().last().unwrap().style.underline);

        let overlapping = RichText::markdown("<b><i>x</b><mark>**y**</mark></i> *after*");
        assert_eq!(overlapping.as_ref(), "<b><i>x</b>y</i> after");
        assert!(
            overlapping
                .spans()
                .iter()
                .any(|span| { span.text == "y" && span.style.bold && span.style.highlight })
        );
        assert!(overlapping.spans().last().unwrap().style.italic);

        let source = r#"<b title="**literal** &amp;">**bold**</b> <!-- *literal* --> *after*"#;
        let parsed = RichText::markdown(source);
        assert_eq!(
            parsed.as_ref(),
            r#"<b title="**literal** &amp;">bold</b> <!-- *literal* --> after"#
        );
        assert!(
            parsed
                .spans()
                .iter()
                .any(|span| span.text == "bold" && span.style.bold)
        );
        assert!(parsed.spans().last().unwrap().style.italic);

        let parsed = RichText::markdown("<b>`**code**` x^2^ $x$ &amp;");
        assert_eq!(parsed.as_ref(), "<b>**code** x^(2) x &");
        assert!(parsed.spans().iter().any(|span| span.style.code));
        assert!(parsed.spans().iter().any(|span| span.formula.is_some()));
    }

    #[test]
    fn emphasis_pairs_can_cross_literal_html_tags() {
        let crossing = RichText::markdown("**<b>foo**</b> after");
        assert_eq!(crossing.as_ref(), "<b>foo</b> after");
        assert_eq!(crossing.spans()[0].text, "<b>foo");
        assert!(crossing.spans()[0].style.bold);
        assert!(!crossing.spans().last().unwrap().style.bold);
        let crossing = RichText::markdown("<b>**foo</b> after**");
        assert_eq!(crossing.as_ref(), "<b>foo</b> after");
        assert!(!crossing.spans()[0].style.bold);
        assert_eq!(crossing.spans().last().unwrap().text, "foo</b> after");
        assert!(crossing.spans().last().unwrap().style.bold);
        let mixed = RichText::markdown("*good* **<b>foo**</b> after *fine*");
        assert_eq!(mixed.as_ref(), "good <b>foo</b> after fine");
        assert!(mixed.spans().first().unwrap().style.italic);
        assert!(mixed.spans().last().unwrap().style.italic);
        let surrounding = RichText::markdown("**hello <b>x</b> world**");
        assert_eq!(surrounding.as_ref(), "hello <b>x</b> world");
        assert!(surrounding.spans().iter().all(|span| span.style.bold));
        for source in ["*<b>foo*</b>", "~~<b>foo~~</b>"] {
            let parsed = RichText::markdown(source);
            assert_eq!(parsed.as_ref(), "<b>foo</b>");
            assert!(parsed.spans()[0].style.italic || parsed.spans()[0].style.strikethrough);
            assert_eq!(parsed.spans().last().unwrap().style, Style::default());
        }
    }

    #[test]
    fn noncanonical_html_tags_keep_their_spelling_and_allow_markdown() {
        for (source, expected) in [
            ("<B>**warning**</b>", "<B>warning</b>"),
            ("<b>**warning**</B>", "<b>warning</B>"),
            ("<u>**foo**</u >", "<u>foo</u >"),
            ("<u>**foo**</U>", "<u>foo</U>"),
            ("<mark>*foo*</MARK>", "<mark>foo</MARK>"),
            (
                "<color name=\"red\">**foo**</color >",
                "<color name=\"red\">foo</color >",
            ),
            ("<b><B>**one**</b> **two**</b>", "<b><B>one</b> two</b>"),
        ] {
            let parsed = RichText::markdown(source);
            assert_eq!(parsed.as_ref(), expected, "{source}");
            assert!(
                parsed
                    .spans()
                    .iter()
                    .any(|span| span.style.bold || span.style.italic)
            );
            assert!(parsed.spans().iter().all(|span| !span.style.underline
                && !span.style.highlight
                && span.style.color.is_none()));
        }
        let parsed = RichText::markdown("<u><B>**raw**</b> **bold**</u>");
        assert_eq!(parsed.as_ref(), "<B>raw</b> bold");
        assert!(parsed.spans().iter().all(|span| span.style.underline));
        assert!(parsed.spans().last().unwrap().style.bold);
    }

    #[test]
    fn nested_emphasis_around_html_follows_markdown() {
        for (source, expected) in [
            ("**<b>foo*</b>*", "<b>foo</b>"),
            ("**x <b>*foo**</b> after*", "x <b>foo</b> after"),
        ] {
            let parsed = RichText::markdown(source);
            assert_eq!(parsed.as_ref(), expected);
            assert!(parsed.spans().iter().all(|span| span.style.italic));
        }
    }

    #[test]
    fn quotes_preserve_whitespace_before_and_after_the_marker() {
        for (source, expected) in [
            ("   > **quote**  ", "   “quote  ”"),
            (" >   *quote*", " “  quote”"),
            ("  >\t quote", "  “ quote”"),
        ] {
            assert_eq!(RichText::markdown(source).as_ref(), expected);
        }
    }

    #[test]
    fn tex_colors_render_and_keep_explicit_black_distinct_from_inherited_color() {
        for body in [
            r"\color{red}x",
            r"\textcolor{blue}{x}",
            r"\colorbox{yellow}{x}",
            r"\fcolorbox{blue}{yellow}{x}",
            r"\definecolor{foo}{rgb}{1,0,0}x",
            r"\definecolor{foo}{rgb}{1,0,0}\textcolor{foo}{x}",
        ] {
            let text = RichText::markdown(&format!("${body}$"));
            assert!(text.spans()[0].formula.is_some(), "{body}");
        }
        let text = RichText::markdown(r"$\textcolor{black}{x}+\frac{1}{2}$");
        let svg = text.spans()[0].formula.as_ref().unwrap().svg_body();
        assert!(svg.contains("fill=\"inherit\""));
        assert!(svg.contains("<g fill=\"#000000\" stroke=\"#000000\">"));

        let text = RichText::markdown(r"$\definecolor{foo}{HTML}{010203}\textcolor{foo}{x}$");
        let svg = text.spans()[0].formula.as_ref().unwrap().svg_body();
        assert!(svg.contains("fill=\"inherit\""));
        assert!(svg.contains("<g fill=\"#010203\" stroke=\"#010203\">"));
    }

    #[test]
    fn underlined_formula_reserves_space_below_the_baseline() {
        let formula = RichText::markdown(r"$\frac{1}{x}$");
        let underlined = RichText::markdown(r"<u>$\frac{1}{x}$</u>");
        assert!(line_ink(underlined.spans(), 14).1 > line_ink(formula.spans(), 14).1);
    }

    #[test]
    fn formula_effects_keep_their_style_and_measured_bounds() {
        for (source, matches) in [
            (
                "**$x$**",
                Style {
                    bold: true,
                    ..Style::default()
                },
            ),
            (
                "*$x$*",
                Style {
                    italic: true,
                    ..Style::default()
                },
            ),
            (
                "~~$x$~~",
                Style {
                    strikethrough: true,
                    ..Style::default()
                },
            ),
            (
                "> $x$",
                Style {
                    quote: true,
                    ..Style::default()
                },
            ),
            (
                "^$x$^",
                Style {
                    script: Some(Script::Superscript),
                    ..Style::default()
                },
            ),
            (
                "~$x$~",
                Style {
                    script: Some(Script::Subscript),
                    ..Style::default()
                },
            ),
        ] {
            let parsed = RichText::markdown(source);
            assert!(
                parsed
                    .spans()
                    .iter()
                    .any(|span| span.formula.is_some() && span.style == matches),
                "{source}"
            );
        }
        let plain = RichText::markdown("$x$");
        let superscript = RichText::markdown("^$x$^");
        let subscript = RichText::markdown("~$x$~");
        assert!(text_width(&superscript, 14) < text_width(&plain, 14));
        assert!(text_width(&subscript, 14) < text_width(&plain, 14));
        assert!(line_ink(superscript.spans(), 14).0 >= line_ink(plain.spans(), 14).0);
        assert!(line_ink(subscript.spans(), 14).1 > line_ink(plain.spans(), 14).1);
    }

    #[test]
    fn wrapping_preserves_spans_and_graphemes() {
        let text = RichText::markdown("**REJECT THE WWWWIDE** application 👩‍💻e\u{301}");
        let lines = wrap_text(&text, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        assert_eq!(joined(&lines, ""), text.as_ref());
        assert!(
            lines
                .iter()
                .flat_map(RichText::spans)
                .any(|span| span.style.bold)
        );
        for line in &lines {
            assert!(text_width(line, LABEL_FONT) <= NODE_LABEL_WIDTH);
        }
        for source in ["**e**\u{301}", "**👩**&zwj;💻"] {
            let mut text = RichText::markdown(source);
            let lines = wrap_text(&text, 1, LABEL_FONT);
            assert_eq!(
                lines.len(),
                1,
                "{source}: a grapheme cannot be split by formatting"
            );
            text.pop_grapheme();
            assert!(
                text.as_ref().is_empty(),
                "{source}: shortening removes the whole grapheme"
            );
        }
    }

    #[test]
    fn math_uses_measured_script_sizes_and_clips_oversized_formulas() {
        let text = RichText::markdown("$x^2$");
        let formula = text.spans()[0].formula.as_ref().unwrap();
        let mut scales = formula.svg_body().split("scale(").skip(1).map(|path| {
            path.split_whitespace()
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap()
        });
        let base = scales.next().unwrap();
        let script = scales.next().unwrap();
        assert!(
            script > 0.0 && script < base,
            "the exponent must render smaller than its base"
        );
        let long = RichText::markdown(&format!("${}$", "a + ".repeat(100) + "b"));
        let lines = wrap_text(&long, NODE_LABEL_WIDTH, LABEL_FONT);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref(), long.as_ref());
        assert_eq!(text_width(&lines[0], LABEL_FONT), NODE_LABEL_WIDTH);
        assert!(text_width(&long, LABEL_FONT) > NODE_LABEL_WIDTH);
        assert!(lines[0].spans()[0].formula.as_ref().unwrap().is_clipped());
    }

    #[test]
    fn math_above_the_label_height_limit_falls_back_to_source() {
        let normal = RichText::markdown(r"$\hspace{1em}\rule{1em}{12em}$");
        assert!(normal.spans()[0].formula.is_some());

        for source in [
            r"$x\rule{1em}{12.01em}$",
            r"$\frac{x\rule{1em}{10em}}{x\rule{1em}{10em}}$",
            r"$\frac{x}{x\rule{1em}{100em}}$",
        ] {
            let oversized = RichText::markdown(source);
            assert_eq!(oversized.as_ref(), source);
            assert!(oversized.spans().iter().all(|span| span.formula.is_none()));
        }
    }

    #[test]
    fn non_finite_math_geometry_falls_back_to_source() {
        for source in [
            r"$\hspace{1.2.3em}x$",
            r"$\hspace{999999999999999999999999999999999999999999em}x$",
            r"$\frac{\hspace{1.2.3em}x}{y}$",
            r"$\sqrt{\hspace{1.2.3em}x}$",
        ] {
            let text = RichText::markdown(source);
            assert_eq!(text.as_ref(), source);
            assert!(text.spans().iter().all(|span| span.formula.is_none()));
        }
        let text = RichText::markdown(r"$\text{NaN}$");
        assert!(text.spans()[0].formula.is_some());
    }

    #[test]
    fn over_nested_math_falls_back_to_source() {
        for (open, close) in [
            ("{", "}"),
            (r"\frac{1}{", "}"),
            (r"\sqrt{", "}"),
            ("x^{", "}"),
        ] {
            let source = format!("${}x{}$", open.repeat(1000), close.repeat(1000));
            let text = RichText::markdown(&source);
            assert_eq!(text.as_ref(), source);
            assert!(text.spans().iter().all(|span| span.formula.is_none()));
        }
    }

    #[test]
    fn width_only_math_keeps_its_advance_without_visible_ink() {
        let text = RichText::markdown(r"a$\,$b");
        assert_eq!(text.as_ref(), r"a\,b");
        assert!(text.spans()[1].formula.is_some());
        assert_eq!(text_width(&RichText::markdown(r"$\,$"), LABEL_FONT), 3);
    }

    #[test]
    fn svg_dimensions_round_and_never_collapse_a_nonzero_side() {
        for (numerator, denominator, expected) in [
            (0, 1, "0"),
            (-1240, 10, "-124"),
            (1, 3, "0.3333"),
            (2, 3, "0.6667"),
            (-2, 3, "-0.6667"),
            (99_999, 100_000, "1"),
            (1, 10_000, "0.0001"),
        ] {
            assert_eq!(
                svg_dimension(&Dim::ratio(numerator, denominator)),
                expected,
                "{numerator}/{denominator}"
            );
        }
        // Too small for four digits, so the exact expansion stands rather than a
        // zero side that would blank the formula it measures.
        let tiny = svg_dimension(&Dim::ratio(1, 100_000));
        assert_ne!(tiny, "0");
        assert_eq!(Dim::parse(&tiny), Dim::ratio(1, 100_000));
    }

    /// Long words split only at grapheme boundaries, even below one cluster's width.
    #[test]
    fn wrapping_splits_between_graphemes_and_never_inside_one() {
        let word = "encyclopædia";
        let lines = wrap_literal(word, 20, 14);

        assert!(lines.len() > 1);
        assert_eq!(joined(&lines, ""), word);
        for line in &lines {
            assert!(
                text_width(line, 14) <= 20,
                "line exceeds: {}",
                line.as_ref()
            );
        }

        // Narrower than one cluster, so only the cluster boundary can break.
        for (source, expected) in [
            ("👩‍💻👩‍💻", ["👩‍💻", "👩‍💻"]),
            ("e\u{301}e\u{301}", ["e\u{301}", "e\u{301}"]),
        ] {
            let lines = wrap_literal(source, 8, 14);
            assert_eq!(
                lines.iter().map(RichText::as_ref).collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn wrapping_preserves_authored_whitespace() {
        let text = "  exact  spacing  ";
        assert_eq!(joined(&wrap_literal(text, 40, 14), ""), text);

        // Markdown consumes its delimiters and nothing else.
        let styled = RichText::markdown("  **exact**  spacing  ");
        assert_eq!(
            joined(&wrap_text(&styled, 40, 14), ""),
            "  exact  spacing  "
        );
    }

    /// The space after a long word sits past the budget, so breaking there
    /// would draw the word and the space outside the node.
    #[test]
    fn wrapping_splits_a_long_word_rather_than_reaching_the_space_after_it() {
        let text = format!("{} tail", "W".repeat(32));
        let lines = wrap_literal(&text, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        assert_eq!(joined(&lines, ""), text);
        for line in &lines {
            assert!(
                text_width(line, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {}",
                line.as_ref()
            );
        }
    }

    #[test]
    fn explicit_newlines_break_lines_whatever_terminates_them() {
        for source in ["one\ntwo", "one\r\ntwo"] {
            let lines = wrap_text(&RichText::markdown(source), NODE_LABEL_WIDTH, LABEL_FONT);
            assert_eq!(
                lines.iter().map(RichText::as_ref).collect::<Vec<_>>(),
                ["one", "two"],
                "{source:?}"
            );
        }
        // An empty authored line keeps a display line of its own.
        let lines = wrap_text(
            &RichText::markdown("one\n\ntwo"),
            NODE_LABEL_WIDTH,
            LABEL_FONT,
        );
        assert_eq!(
            lines.iter().map(RichText::as_ref).collect::<Vec<_>>(),
            ["one", "", "two"]
        );
    }

    #[test]
    fn underscore_emphasis_follows_markdown_and_leaves_identifiers_alone() {
        for (source, expected) in [
            ("__bold__ and _italic_", "bold and italic"),
            ("some_identifier stays", "some_identifier stays"),
            ("snake_case_name", "snake_case_name"),
        ] {
            assert_eq!(RichText::markdown(source).as_ref(), expected, "{source}");
        }
        let text = RichText::markdown("__bold__ and _italic_");
        assert!(
            text.spans()
                .iter()
                .any(|span| span.text == "bold" && span.style.bold)
        );
        assert!(
            text.spans()
                .iter()
                .any(|span| span.text == "italic" && span.style.italic)
        );
        assert!(
            RichText::markdown("some_identifier stays")
                .spans()
                .iter()
                .all(|span| span.style == Style::default())
        );
    }

    #[test]
    fn every_palette_color_reaches_its_own_class() {
        for (name, color, class) in [
            ("red", Color::Red, "md-color-red"),
            ("green", Color::Green, "md-color-green"),
            ("blue", Color::Blue, "md-color-blue"),
            ("purple", Color::Purple, "md-color-purple"),
            ("muted", Color::Muted, "md-color-muted"),
        ] {
            let text = RichText::markdown(&format!(r#"<color name="{name}">tinted</color>"#));
            assert_eq!(text.as_ref(), "tinted", "{name}");
            assert_eq!(text.spans()[0].style.color, Some(color), "{name}");
            assert_eq!(color.class(), class);
        }
    }

    /// A parser result that holds no text of its own must not swallow the line.
    #[test]
    fn document_structure_other_than_a_paragraph_stays_literal() {
        for source in [
            "[ref]: http://example.com",
            "- **item**",
            "1. **item**",
            "---",
            "    indented **code**",
            "# **heading**",
            "<div>**warning**</div>",
        ] {
            assert_eq!(RichText::markdown(source).as_ref(), source, "{source}");
        }
    }

    #[test]
    fn display_math_lays_out_taller_than_the_same_formula_inline() {
        let inline = RichText::markdown(r"$\sum_{n=1}^{5} n$");
        let display = RichText::markdown(r"$$\sum_{n=1}^{5} n$$");
        let ink = |text: &RichText| {
            let (ascent, descent) = line_ink(text.spans(), LABEL_FONT);
            ascent + descent
        };

        assert!(inline.spans()[0].formula.is_some());
        assert!(display.spans()[0].formula.is_some());
        assert!(
            ink(&display) > ink(&inline),
            "display math must be taller than the same formula inline"
        );
    }

    #[test]
    fn shortening_keeps_the_remaining_effects_and_an_unstyled_ellipsis() {
        let mut text = RichText::markdown("**Bold** and `code`");
        for _ in 0..4 {
            text.pop_grapheme();
        }
        assert_eq!(text.as_ref(), "Bold and ");
        assert!(text.spans().iter().any(|span| span.style.bold));
        text.push_literal("…");
        assert_eq!(
            text.spans()
                .last()
                .expect("the ellipsis was appended")
                .style,
            Style::default()
        );

        // A formula is indivisible, so shortening drops it whole.
        let mut math = RichText::markdown("a $x^2$");
        math.pop_grapheme();
        assert_eq!(math.as_ref(), "a ");
    }
}
