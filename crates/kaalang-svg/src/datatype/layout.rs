//! Lays out a type diagram: one row of incoming cards above the focus card, and
//! a route for each relationship drawn into the focus.
//!
//! Routes never cross, and run inside a card only along a groove. An incoming
//! card lists its tiles one under another, so each reaches the card's right
//! side: a line starts there, level with its target, and joins its column's
//! trunk, which drops into the focus. The focus sets a struct's tiles side
//! by side, each row widened to the card's edge, and is widened to reach under
//! every trunk. Its own lines leave each tile where it touches the card's edge:
//! a struct's from the bottom edge into one bus below the card, a union's from
//! the right side. An enum variant's tiles do not all reach an edge, so each
//! variant ends in a groove, used or not, whose bus leaves the focus through a
//! port on its right side. All of them join one trunk there, which climbs and
//! arrives at the focus's top edge right of every incoming line.

use std::num::NonZeroUsize;

use kaalang_compiler::geometry::Point;

use super::{Card, Structure, Tile};
use crate::{
    markup::MARGIN,
    text::{
        LABEL_FONT, LINE_HEIGHT, RichText, SECONDARY_FONT, Style, block_metrics, text_width,
        wrap_text,
    },
};

pub(super) const NAME_FONT: i32 = 18;
const NAME_LINE_HEIGHT: i32 = 24;
const CODE_LINE_HEIGHT: i32 = 16;
const CARD_PADDING_X: i32 = 14;
const CARD_PADDING_Y: i32 = 12;
/// Between a card's name and its kind on the same line.
const HEADER_GAP: i32 = 24;
const DESCRIPTION_GAP: i32 = 4;
/// The widest a description runs before it wraps, unless the structure is wider.
const DESCRIPTION_WIDTH: i32 = 420;
const VARIANT_GAP: i32 = 12;
/// Tile text starts where the card's own text does.
const TILE_PADDING_X: i32 = CARD_PADDING_X;
const TILE_PADDING_Y: i32 = 8;
const TILE_GAP: i32 = 6;
const TILE_TEXT_MIN: i32 = 100;
/// The widest a tile grows for its own source text, which wraps past it.
const TILE_TEXT_MAX: i32 = 320;
/// The widest a tile grows for its description before the description wraps.
const TILE_DESCRIPTION_WIDTH: i32 = 160;
/// From the start of a target's text to the line that leaves under its arrow.
const ANCHOR_INSET: i32 = 4;
/// The spacing of parallel lines.
const LANE: i32 = 10;
/// From a card's side to the first line beside it.
const CHANNEL_INSET: i32 = 14;
/// The strip under an enum variant, whose lines leave through a bus at its
/// middle.
const GROOVE: i32 = 2 * LANE;
const CARD_GAP: i32 = 32;
/// The widest the incoming cards run before they wrap into another row: a
/// Full HD screen less the margins around the page, unless the focus is wider.
const ROW_WIDTH: i32 = 1800;
/// How far a variant's tiles sit in from its name in a listed card.
const VARIANT_INDENT: i32 = 16;
/// Between a listed tile's name and type and its description.
const COLUMN_GAP: i32 = 16;
/// The narrowest a listed card wraps its description.
const LIST_DESCRIPTION_MIN: i32 = 240;
/// Between the incoming row and the focus, before the focus's own lines.
const CORRIDOR: i32 = 48;

pub(super) struct TypeScene {
    pub(super) width: i32,
    pub(super) height: i32,
    /// The incoming cards in source order, then the focus.
    pub(super) cards: Vec<CardShape>,
    pub(super) routes: Vec<Route>,
    /// Where each bus along a groove of the focus leaves it, on its right side.
    pub(super) ports: Vec<Point>,
}

pub(super) struct CardShape {
    /// The card's name, as its header shows it.
    pub(super) name: String,
    pub(super) focus: bool,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
    /// The separators between the card's header, rows, and tiles.
    pub(super) rules: Vec<[Point; 2]>,
    /// Each groove as left, top, right, and bottom.
    pub(super) grooves: Vec<Bounds>,
    pub(super) texts: Vec<TextBlock>,
}

pub(super) type Bounds = (i32, i32, i32, i32);

/// One block of text, written from the top-left of its first line.
pub(super) struct TextBlock {
    pub(super) text: Text,
    pub(super) x: i32,
    pub(super) top: i32,
    pub(super) lines: Vec<RichText>,
}

/// The kinds of text a card shows, each with its own typography.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Text {
    Name,
    /// The form and representation, right-aligned on the name's line.
    Kind,
    /// Authored Markdown.
    Description,
    Field,
    Type,
    Variant,
    /// A relationship's target, right-aligned at the bottom of its tile.
    Target,
}

impl Text {
    pub(super) const fn font(self) -> i32 {
        match self {
            Self::Name => NAME_FONT,
            Self::Kind | Self::Field | Self::Type | Self::Target => SECONDARY_FONT,
            Self::Description | Self::Variant => LABEL_FONT,
        }
    }

    /// Whether the text is placed by its right end.
    pub(super) const fn right_aligned(self) -> bool {
        matches!(self, Self::Kind | Self::Target)
    }

    pub(super) const fn line_height(self) -> i32 {
        match self {
            Self::Name => NAME_LINE_HEIGHT,
            Self::Field | Self::Type | Self::Target => CODE_LINE_HEIGHT,
            Self::Kind | Self::Description | Self::Variant => LINE_HEIGHT,
        }
    }

    /// The style the text is measured in, matching its stylesheet rule.
    fn style(self) -> Style {
        match self {
            Self::Name => Style {
                bold: true,
                ..Style::default()
            },
            // Rust's own names are set in code.
            Self::Field | Self::Type | Self::Variant => Style {
                code: true,
                ..Style::default()
            },
            Self::Kind | Self::Description | Self::Target => Style::default(),
        }
    }

    fn parse(self, text: &str) -> RichText {
        if self == Self::Description {
            RichText::markdown(text)
        } else {
            RichText::marked(text, self.style())
        }
    }

    fn width(self, text: &RichText) -> i32 {
        text_width(text, self.font())
    }

    /// Wraps at `budget`. Literal lines drop the style they were measured in,
    /// which their stylesheet rule supplies instead, and keep their visibility
    /// badge; a description and a target mix styles and keep them.
    fn wrap(self, text: &RichText, budget: i32) -> Vec<RichText> {
        let lines = wrap_text(text, budget.max(1), self.font());
        if matches!(self, Self::Description | Self::Target) {
            lines
        } else {
            lines
                .iter()
                .map(|line| RichText::marked(line.as_ref(), Style::default()))
                .collect()
        }
    }

    /// Wraps a text a tile may lack; an absent text has no lines.
    fn wrap_optional(self, text: Option<&RichText>, budget: i32) -> Vec<RichText> {
        text.map_or_else(Vec::new, |text| self.wrap(text, budget))
    }

    fn height(self, lines: &[RichText]) -> i32 {
        block_metrics(lines, self.font(), self.line_height()).height
    }

    fn first_baseline(self, lines: &[RichText]) -> i32 {
        block_metrics(lines, self.font(), self.line_height()).baselines[0]
    }
}

/// One relationship's line, from its source tile into the focus.
pub(super) struct Route {
    /// The index of the source card in [`TypeScene::cards`].
    pub(super) card: usize,
    pub(super) points: Vec<Point>,
}

/// Lays out the focus under its incoming cards, in groups that each start a
/// new row, with `columns` cards to a row when given.
pub(super) fn layout(groups: &[&[Card]], focus: &Card, columns: Option<NonZeroUsize>) -> TypeScene {
    let sized = SizedCard::new(focus);
    let natural = sized.width();
    let Row {
        mut cards,
        mut routes,
        bottom: row_bottom,
        right,
    } = row(groups, columns, ROW_WIDTH.max(natural));
    let incoming = cards.len();
    let mut ports = Vec::new();
    let rightmost_drop = routes
        .iter()
        .map(|route| route.points.last().expect("a route starts at its tile").x)
        .max();

    let own = sized
        .tiles()
        .iter()
        .any(|tile| tile.drawn && tile.target.is_some());
    let top = if incoming == 0 {
        MARGIN + if own { CHANNEL_INSET + LANE } else { 0 }
    } else {
        row_bottom + CORRIDOR
    };
    // The focus's own trunk arrives right of every incoming one, so that it
    // crosses none of them.
    let entry = (MARGIN + natural - CHANNEL_INSET)
        .max(rightmost_drop.map_or(MARGIN + CHANNEL_INSET, |x| x + CHANNEL_INSET));
    let width = [
        natural,
        right - MARGIN,
        rightmost_drop.map_or(0, |x| x + CHANNEL_INSET - MARGIN),
        if own {
            entry + CHANNEL_INSET - MARGIN
        } else {
            0
        },
    ]
    .into_iter()
    .max()
    .expect("the candidates are not empty");
    let built = sized.build(natural, width);
    let around = Around {
        origin: Point { x: MARGIN, y: top },
        right: MARGIN + width,
        entry,
    };

    for route in &mut routes {
        let last = *route.points.last().expect("a route starts at its tile");
        route.points.push(Point { x: last.x, y: top });
    }
    for anchor in &built.anchors {
        routes.push(Route {
            card: incoming,
            points: around.route(anchor, anchor.bus.map(|index| built.buses[index])),
        });
    }
    for &bus in &built.buses {
        // A bus below the card leaves it through no port.
        if bus < built.height {
            ports.push(Point {
                x: around.right,
                y: top + bus,
            });
        }
    }
    let bottom = built.buses.iter().copied().fold(built.height, i32::max);
    let height = top + bottom + MARGIN;
    cards.push(built.shape(focus, around.origin, width, true));

    TypeScene {
        width: MARGIN + width + if own { CHANNEL_INSET } else { 0 } + MARGIN,
        height,
        cards,
        routes,
        ports,
    }
}

/// The incoming cards in rows, each route stopped on its column's trunk, where
/// it turns down toward the focus.
struct Row {
    cards: Vec<CardShape>,
    routes: Vec<Route>,
    bottom: i32,
    right: i32,
}

/// Sets the incoming cards side by side in source order, each group from a new
/// row, wrapping into further rows after `columns` cards, or else into rows no
/// wider than `limit`. The cards of a column share its width and one trunk,
/// which drops through the gap right of the column into the focus.
fn row(groups: &[&[Card]], columns: Option<NonZeroUsize>, limit: i32) -> Row {
    let incoming: Vec<&Card> = groups.iter().flat_map(|group| group.iter()).collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    let sized: Vec<SizedCard> = incoming.iter().map(|card| SizedCard::new(card)).collect();
    let natural: Vec<i32> = sized.iter().map(SizedCard::list_width).collect();
    let widest = lengths.iter().copied().max().unwrap_or(0);
    let grid = match columns {
        Some(columns) => Grid::new(&lengths, &natural, columns.get()),
        None => (1..=widest)
            .rev()
            .map(|columns| Grid::new(&lengths, &natural, columns))
            .find(|grid| grid.width() <= limit)
            .unwrap_or_else(|| Grid::new(&lengths, &natural, 1)),
    };
    let built: Vec<Built> = sized
        .iter()
        .zip(&grid.cells)
        .map(|(card, &(_, column))| card.build_list(grid.widths[column]))
        .collect();
    let rows = grid.cells.last().map_or(0, |&(line, _)| line + 1);
    let mut tops = vec![MARGIN];
    for line in 0..rows {
        let height = built
            .iter()
            .zip(&grid.cells)
            .filter(|(_, cell)| cell.0 == line)
            .map(|(card, _)| card.height)
            .max()
            .unwrap_or(0);
        tops.push(tops[line] + height + CARD_GAP);
    }

    let mut row = Row {
        cards: Vec::with_capacity(incoming.len() + 1),
        routes: Vec::new(),
        bottom: MARGIN,
        right: MARGIN,
    };
    for (index, (card, built)) in incoming.iter().zip(built).enumerate() {
        let (line, column) = grid.cells[index];
        let width = grid.widths[column];
        let origin = Point {
            x: grid.lefts[column],
            y: tops[line],
        };
        let trunk = origin.x + width + CHANNEL_INSET;
        for anchor in &built.anchors {
            let at = offset(anchor.at, origin);
            row.routes.push(Route {
                card: index,
                points: vec![at, Point { x: trunk, y: at.y }],
            });
        }
        row.bottom = row.bottom.max(origin.y + built.height);
        row.right = row.right.max(origin.x + width);
        row.cards.push(built.shape(card, origin, width, false));
    }
    row
}

/// Where each incoming card goes: each group from a new row, and within it
/// the `index`th card in column `index % columns`.
struct Grid {
    /// Each card's row and column.
    cells: Vec<(usize, usize)>,
    /// Each column's width, its widest card's.
    widths: Vec<i32>,
    lefts: Vec<i32>,
}

impl Grid {
    fn new(sizes: &[usize], natural: &[i32], columns: usize) -> Self {
        let mut cells = Vec::with_capacity(natural.len());
        let mut first = 0;
        for &size in sizes {
            cells.extend((0..size).map(|index| (first + index / columns, index % columns)));
            first += size.div_ceil(columns);
        }
        let used = cells
            .iter()
            .map(|&(_, column)| column + 1)
            .max()
            .unwrap_or(1);
        let mut widths = vec![0; used];
        for (&(_, column), width) in cells.iter().zip(natural) {
            widths[column] = widths[column].max(*width);
        }
        let mut grid = Self {
            cells,
            widths,
            lefts: Vec::with_capacity(used),
        };
        let mut x = MARGIN;
        for column in 0..used {
            grid.lefts.push(x);
            x += grid.widths[column] + CARD_GAP;
        }
        grid
    }

    /// From the left margin to the last column's trunk.
    fn width(&self) -> i32 {
        let last = self.widths.len() - 1;
        self.lefts[last] + self.widths[last] + CHANNEL_INSET - MARGIN
    }
}

/// Where the focus's lines to itself run: down to a bus and along it, or
/// straight from the card's right side, to the trunk beside the card, then up
/// and over its top edge.
struct Around {
    origin: Point,
    right: i32,
    entry: i32,
}

impl Around {
    fn route(&self, anchor: &Anchor, bus: Option<i32>) -> Vec<Point> {
        let at = offset(anchor.at, self.origin);
        let trunk = self.right + CHANNEL_INSET;
        let corridor = self.origin.y - CHANNEL_INSET;
        let mut points = vec![at];
        if let Some(bus) = bus {
            let y = self.origin.y + bus;
            points.extend([Point { x: at.x, y }, Point { x: trunk, y }]);
        } else {
            points.push(Point { x: trunk, y: at.y });
        }
        points.extend([
            Point {
                x: trunk,
                y: corridor,
            },
            Point {
                x: self.entry,
                y: corridor,
            },
            Point {
                x: self.entry,
                y: self.origin.y,
            },
        ]);
        points
    }
}

const fn offset(point: Point, origin: Point) -> Point {
    Point {
        x: origin.x + point.x,
        y: origin.y + point.y,
    }
}

/// A card's text parsed and measured once, for both of its builds.
struct SizedCard {
    name: RichText,
    kind: RichText,
    description: Option<RichText>,
    structure: SizedStructure,
}

enum SizedStructure {
    Empty,
    Row(Vec<SizedTile>),
    Variants(Vec<SizedVariant>),
    Stack(Vec<SizedTile>),
}

struct SizedVariant {
    name: RichText,
    case: Option<RichText>,
    tiles: Vec<SizedTile>,
}

/// A tile's text, parsed and measured once.
struct SizedTile {
    /// The width the tile needs for its own text, side by side with others.
    width: i32,
    /// The width of its name and type, listed.
    code: i32,
    /// The width of its description and target, listed.
    text: i32,
    label: Option<RichText>,
    ty: RichText,
    description: Option<RichText>,
    target: Option<RichText>,
    drawn: bool,
}

impl SizedTile {
    fn new(tile: &Tile) -> Self {
        let label = tile.label.as_deref().map(|label| Text::Field.parse(label));
        let ty = Text::Type.parse(&tile.ty);
        let description = tile
            .description
            .as_deref()
            .map(|text| Text::Description.parse(text));
        // The target is a Rust path, set in code like the type above it, after
        // a triangle that stays legible at its size where an arrow does not.
        let target = tile.target.as_deref().map(|target| {
            let mut text = RichText::literal("▸ ");
            text.push_styled(target, Text::Type.style());
            text
        });
        let code = label
            .as_ref()
            .map_or(0, |label| Text::Field.width(label))
            .max(Text::Type.width(&ty))
            .min(TILE_TEXT_MAX);
        let text = description
            .as_ref()
            .map_or(0, |description| {
                Text::Description
                    .width(description)
                    .min(TILE_DESCRIPTION_WIDTH)
            })
            .max(
                target
                    .as_ref()
                    .map_or(0, |target| Text::Target.width(target)),
            );
        Self {
            width: code.max(text).clamp(TILE_TEXT_MIN, TILE_TEXT_MAX) + 2 * TILE_PADDING_X,
            code,
            text,
            label,
            ty,
            description,
            target,
            drawn: tile.drawn,
        }
    }

    fn wrap(&self, width: i32) -> WrappedTile {
        let budget = width - 2 * TILE_PADDING_X;
        WrappedTile {
            width,
            label: Text::Field.wrap_optional(self.label.as_ref(), budget),
            ty: Text::Type.wrap(&self.ty, budget),
            description: Text::Description.wrap_optional(self.description.as_ref(), budget),
            target: Text::Target.wrap_optional(self.target.as_ref(), budget),
            drawn: self.drawn,
        }
    }
}

/// A tile wrapped at the width it is drawn at.
struct WrappedTile {
    width: i32,
    label: Vec<RichText>,
    ty: Vec<RichText>,
    description: Vec<RichText>,
    target: Vec<RichText>,
    drawn: bool,
}

impl WrappedTile {
    /// The height this tile needs, with its target under the rest.
    fn height(&self) -> i32 {
        let gap = |lines: &[RichText]| if lines.is_empty() { 0 } else { TILE_GAP };
        2 * TILE_PADDING_Y
            + Text::Field.height(&self.label)
            + Text::Type.height(&self.ty)
            + gap(&self.description)
            + Text::Description.height(&self.description)
            + gap(&self.target)
            + Text::Target.height(&self.target)
    }
}

impl SizedCard {
    fn new(card: &Card) -> Self {
        let tiles = |tiles: &[Tile]| tiles.iter().map(SizedTile::new).collect();
        Self {
            name: Text::Name.parse(&card.name),
            kind: Text::Kind.parse(&card.kind),
            description: card
                .description
                .as_deref()
                .map(|text| Text::Description.parse(text)),
            structure: match &card.structure {
                Structure::Empty => SizedStructure::Empty,
                Structure::Row(row) => SizedStructure::Row(tiles(row)),
                Structure::Stack(stack) => SizedStructure::Stack(tiles(stack)),
                Structure::Variants(variants) => SizedStructure::Variants(
                    variants
                        .iter()
                        .map(|variant| SizedVariant {
                            name: Text::Variant.parse(&variant.name),
                            case: variant
                                .case
                                .as_deref()
                                .map(|text| Text::Description.parse(text)),
                            tiles: tiles(&variant.tiles),
                        })
                        .collect(),
                ),
            },
        }
    }

    fn tiles(&self) -> Vec<&SizedTile> {
        match &self.structure {
            SizedStructure::Empty => Vec::new(),
            SizedStructure::Row(tiles) | SizedStructure::Stack(tiles) => tiles.iter().collect(),
            SizedStructure::Variants(variants) => {
                variants.iter().flat_map(|variant| &variant.tiles).collect()
            }
        }
    }

    /// The header's width, with its description wrapped at `budget`.
    fn header_width(&self, budget: i32) -> i32 {
        let description = self
            .description
            .as_ref()
            .map_or(0, |text| wrapped(text, budget));
        (Text::Name.width(&self.name) + HEADER_GAP + Text::Kind.width(&self.kind)).max(description)
    }

    /// The widest variant name with its case wrapped at `budget`.
    fn variants_width(&self, budget: i32) -> i32 {
        let SizedStructure::Variants(variants) = &self.structure else {
            return 0;
        };
        variants
            .iter()
            .map(|variant| {
                let case = variant
                    .case
                    .as_ref()
                    .map_or(0, |case| VARIANT_GAP + wrapped(case, budget));
                2 * CARD_PADDING_X + Text::Variant.width(&variant.name) + case
            })
            .max()
            .unwrap_or(0)
    }

    /// An incoming card's width: its tiles one under another, each with its
    /// name and type beside its description.
    fn list_width(&self) -> i32 {
        let tiles = self.tiles();
        let code = tiles.iter().map(|tile| tile.code).max().unwrap_or(0);
        let text = tiles.iter().map(|tile| tile.text).max().unwrap_or(0);
        let indent = if matches!(self.structure, SizedStructure::Variants(_)) {
            VARIANT_INDENT
        } else {
            0
        };
        let list = if tiles.is_empty() {
            0
        } else {
            indent + 2 * TILE_PADDING_X + code + COLUMN_GAP + text
        };
        let budget = if list == 0 {
            DESCRIPTION_WIDTH
        } else {
            (list - 2 * CARD_PADDING_X).max(LIST_DESCRIPTION_MIN)
        };
        (2 * CARD_PADDING_X + self.header_width(budget))
            .max(self.variants_width(budget))
            .max(list)
    }

    /// The focus's own width: its header, its description up to
    /// [`DESCRIPTION_WIDTH`], and its structure.
    fn width(&self) -> i32 {
        let row = |tiles: &[SizedTile]| tiles.iter().map(|tile| tile.width).sum::<i32>();
        let header = self.header_width(DESCRIPTION_WIDTH);
        let structure = match &self.structure {
            SizedStructure::Empty => 0,
            SizedStructure::Row(tiles) => row(tiles),
            SizedStructure::Stack(tiles) => tiles.iter().map(|tile| tile.width).max().unwrap_or(0),
            SizedStructure::Variants(variants) => variants
                .iter()
                .map(|variant| row(&variant.tiles))
                .max()
                .unwrap_or(0)
                .max(self.variants_width(DESCRIPTION_WIDTH)),
        };
        (2 * CARD_PADDING_X + header).max(structure)
    }

    /// Places the card's text and rules from its top-left corner. The header
    /// wraps at the card's own width; tiles widen to `width` and wrap there.
    fn build(&self, natural: i32, width: i32) -> Built {
        let mut built = Built {
            width,
            ..Built::default()
        };
        let mut y = self.header(&mut built, natural);
        match &self.structure {
            SizedStructure::Empty => {}
            SizedStructure::Row(tiles) => {
                built.rule(0, y, width, y);
                y += built.row(tiles, y);
                // The row closes the card, so its lines join below it.
                built.bus(0, y + CHANNEL_INSET);
            }
            SizedStructure::Stack(tiles) => {
                for tile in tiles {
                    built.rule(0, y, width, y);
                    let tile = tile.wrap(width);
                    let height = tile.height();
                    built.tile(&tile, 0, y, height, true);
                    y += height;
                }
            }
            SizedStructure::Variants(variants) => {
                for variant in variants {
                    built.rule(0, y, width, y);
                    y = built.variant(variant, y, natural);
                    let first = built.anchors.len();
                    if !variant.tiles.is_empty() {
                        built.rule(0, y, width, y);
                        y += built.row(&variant.tiles, y);
                    }
                    // Every variant ends in a groove, used or not, so that
                    // the variants read alike.
                    y += built.groove(first, y);
                }
            }
        }
        built.height = y;
        built
    }

    /// An incoming card, its tiles one under another, each reaching the
    /// card's right side.
    fn build_list(&self, width: i32) -> Built {
        let mut built = Built {
            width,
            ..Built::default()
        };
        let code = self.tiles().iter().map(|tile| tile.code).max().unwrap_or(0);
        let mut y = self.header(&mut built, width);
        match &self.structure {
            SizedStructure::Empty => {}
            SizedStructure::Row(tiles) | SizedStructure::Stack(tiles) => {
                for tile in tiles {
                    built.rule(0, y, width, y);
                    y += built.listed(tile, 0, y, code);
                }
            }
            SizedStructure::Variants(variants) => {
                for variant in variants {
                    built.rule(0, y, width, y);
                    y = built.variant(variant, y, width);
                    for tile in &variant.tiles {
                        built.rule(VARIANT_INDENT, y, width, y);
                        y += built.listed(tile, VARIANT_INDENT, y, code);
                    }
                }
            }
        }
        built.height = y;
        built
    }

    /// Places the name, kind, and description, and returns where the
    /// structure starts. The description wraps at `natural`.
    fn header(&self, built: &mut Built, natural: i32) -> i32 {
        let mut y = CARD_PADDING_Y;
        let name = vec![RichText::marked(self.name.as_ref(), Style::default())];
        let kind = vec![RichText::literal(self.kind.as_ref())];
        // The kind shares the name's baseline.
        let kind_top = y + Text::Name.first_baseline(&name) - Text::Kind.first_baseline(&kind);
        y += Text::Name.height(&name);
        built.text(Text::Name, CARD_PADDING_X, CARD_PADDING_Y, name);
        built.text(Text::Kind, built.width - CARD_PADDING_X, kind_top, kind);
        if let Some(description) = &self.description {
            let lines = Text::Description.wrap(description, natural - 2 * CARD_PADDING_X);
            y += DESCRIPTION_GAP;
            built.text(Text::Description, CARD_PADDING_X, y, lines);
        }
        built.bottom(y) + CARD_PADDING_Y
    }
}

/// The width of `text` wrapped as a description at `budget`.
fn wrapped(text: &RichText, budget: i32) -> i32 {
    Text::Description
        .wrap(text, budget)
        .iter()
        .map(|line| Text::Description.width(line))
        .max()
        .unwrap_or(0)
}

/// A card's text, rules, and line anchors, relative to its top-left corner.
#[derive(Default)]
struct Built {
    width: i32,
    height: i32,
    texts: Vec<TextBlock>,
    rules: Vec<[Point; 2]>,
    anchors: Vec<Anchor>,
    /// The height of each bus from the card's top.
    buses: Vec<i32>,
    /// The light strip of every groove.
    strips: Vec<Bounds>,
}

/// Where a drawn relationship's line starts: under its tile's target on the
/// bottom edge of the tile's row, or on the card's right side level with the
/// target.
struct Anchor {
    at: Point,
    /// The bus a line from under its target drops into, set once its row is
    /// placed. A line from the card's side has none.
    bus: Option<usize>,
}

impl Built {
    fn text(&mut self, text: Text, x: i32, top: i32, lines: Vec<RichText>) {
        self.texts.push(TextBlock {
            text,
            x,
            top,
            lines,
        });
    }

    fn rule(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.rules
            .push([Point { x: x1, y: y1 }, Point { x: x2, y: y2 }]);
    }

    /// The lowest text bottom so far, or `y` when the text ends above it.
    fn bottom(&self, y: i32) -> i32 {
        self.texts
            .iter()
            .map(|block| block.top + block.text.height(&block.lines))
            .fold(y, i32::max)
    }

    /// Places a row of tiles side by side across the card, widened evenly to
    /// its edge, with a divider between each two.
    fn row(&mut self, tiles: &[SizedTile], top: i32) -> i32 {
        let natural: i32 = tiles.iter().map(|tile| tile.width).sum();
        let count = i32::try_from(tiles.len()).expect("a row has fewer tiles than i32::MAX");
        let extra = (self.width - natural).max(0);
        let tiles: Vec<WrappedTile> = (0..)
            .zip(tiles)
            .map(|(index, tile)| {
                // The last tile takes the remainder, so the row ends at the edge.
                let share = extra / count + if index + 1 == count { extra % count } else { 0 };
                tile.wrap(tile.width + share)
            })
            .collect();
        let height = tiles.iter().map(WrappedTile::height).max().unwrap_or(0);
        let mut x = 0;
        for tile in &tiles {
            self.tile(tile, x, top, height, false);
            x += tile.width;
            if x < self.width {
                self.rule(x, top, x, top + height);
            }
        }
        height
    }

    /// Places a tile's text. A drawn target's line starts under it, or level
    /// with it on the card's right side when `side`.
    fn tile(&mut self, tile: &WrappedTile, x: i32, top: i32, height: i32, side: bool) {
        let left = x + TILE_PADDING_X;
        let mut y = top + TILE_PADDING_Y;
        for (text, lines, gap) in [
            (Text::Field, &tile.label, 0),
            (Text::Type, &tile.ty, 0),
            (Text::Description, &tile.description, TILE_GAP),
        ] {
            if !lines.is_empty() {
                y += gap;
                self.text(text, left, y, lines.clone());
                y += text.height(lines);
            }
        }
        if let Some(first) = tile.target.first() {
            let bottom = top + height;
            let end = x + tile.width - TILE_PADDING_X;
            let target = bottom - TILE_PADDING_Y - Text::Target.height(&tile.target);
            self.text(Text::Target, end, target, tile.target.clone());
            if tile.drawn {
                let at = if side {
                    Point {
                        x: self.width,
                        y: target + Text::Target.line_height() / 2,
                    }
                } else {
                    Point {
                        x: end - Text::Target.width(first) + ANCHOR_INSET,
                        y: bottom,
                    }
                };
                self.anchors.push(Anchor { at, bus: None });
            }
        }
    }

    /// Places a variant's name and case from `top`, and returns where its
    /// tiles start. The case wraps short of `right`.
    fn variant(&mut self, variant: &SizedVariant, top: i32, right: i32) -> i32 {
        let top = top + TILE_PADDING_Y;
        let name = vec![RichText::literal(variant.name.as_ref())];
        let indent = CARD_PADDING_X + Text::Variant.width(&variant.name) + VARIANT_GAP;
        let mut header = Text::Variant.height(&name);
        self.text(Text::Variant, CARD_PADDING_X, top, name);
        if let Some(case) = &variant.case {
            let lines = Text::Description.wrap(case, right - indent - CARD_PADDING_X);
            header = header.max(Text::Description.height(&lines));
            self.text(Text::Description, indent, top, lines);
        }
        top + header + TILE_PADDING_Y
    }

    /// Places a tile as one row of a listed card, from `indent` to the card's
    /// right side: its name and type in a column `code` wide, then its
    /// description and target, which ends at the tile's padding. A drawn
    /// target's line starts on the card's side, level with it. Returns the
    /// height.
    fn listed(&mut self, tile: &SizedTile, indent: i32, top: i32, code: i32) -> i32 {
        let left = indent + TILE_PADDING_X;
        let column = left + code + COLUMN_GAP;
        let end = self.width - TILE_PADDING_X;
        let budget = end - column;
        let y = top + TILE_PADDING_Y;
        let mut code_bottom = y;
        for (text, lines) in [
            (
                Text::Field,
                Text::Field.wrap_optional(tile.label.as_ref(), code),
            ),
            (Text::Type, Text::Type.wrap(&tile.ty, code)),
        ] {
            if !lines.is_empty() {
                let height = text.height(&lines);
                self.text(text, left, code_bottom, lines);
                code_bottom += height;
            }
        }
        let mut text_bottom = y;
        let description = Text::Description.wrap_optional(tile.description.as_ref(), budget);
        if !description.is_empty() {
            text_bottom += Text::Description.height(&description);
            self.text(Text::Description, column, y, description);
        }
        let target = Text::Target.wrap_optional(tile.target.as_ref(), budget);
        if !target.is_empty() {
            if text_bottom > y {
                text_bottom += TILE_GAP;
            }
            if tile.drawn {
                self.anchors.push(Anchor {
                    at: Point {
                        x: self.width,
                        y: text_bottom + Text::Target.line_height() / 2,
                    },
                    bus: None,
                });
            }
            let height = Text::Target.height(&target);
            self.text(Text::Target, end, text_bottom, target);
            text_bottom += height;
        }
        code_bottom.max(text_bottom) + TILE_PADDING_Y - top
    }

    /// Opens a groove at `top` and returns its height. The lines of the tiles
    /// from `first` on join its bus.
    fn groove(&mut self, first: usize, top: i32) -> i32 {
        self.bus(first, top + GROOVE / 2);
        self.rule(0, top, self.width, top);
        self.strips.push((0, top, self.width, top + GROOVE));
        GROOVE
    }

    /// Lays a bus at `y` for the lines of the tiles from `first` on, if they
    /// draw any.
    fn bus(&mut self, first: usize, y: i32) {
        if self.anchors.len() > first {
            let index = self.buses.len();
            self.buses.push(y);
            for anchor in &mut self.anchors[first..] {
                anchor.bus = Some(index);
            }
        }
    }

    fn shape(self, card: &Card, origin: Point, width: i32, focus: bool) -> CardShape {
        CardShape {
            name: card.name.clone(),
            focus,
            x: origin.x,
            y: origin.y,
            width,
            height: self.height,
            grooves: self
                .strips
                .iter()
                .map(|&(left, top, right, bottom)| {
                    (
                        origin.x + left,
                        origin.y + top,
                        origin.x + right,
                        origin.y + bottom,
                    )
                })
                .collect(),
            rules: self
                .rules
                .into_iter()
                .map(|[from, to]| [offset(from, origin), offset(to, origin)])
                .collect(),
            texts: self
                .texts
                .into_iter()
                .map(|block| TextBlock {
                    x: origin.x + block.x,
                    top: origin.y + block.top,
                    ..block
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use kaalang_compiler::geometry::{Point, bundle_meetings, compatible, enters, overlaps};

    use super::{ROW_WIDTH, TypeScene, layout};
    use crate::{
        datatype::{Card, Structure, Tile, VariantTile},
        markup::MARGIN,
    };

    /// Whether `point` lies in `bounds`, edges included.
    fn holds((left, top, right, bottom): (i32, i32, i32, i32), point: Point) -> bool {
        (left..=right).contains(&point.x) && (top..=bottom).contains(&point.y)
    }

    fn tile(label: &str, target: Option<&str>, drawn: bool) -> Tile {
        Tile {
            label: Some(label.to_owned()),
            ty: "MemberId".to_owned(),
            description: Some("A description that wraps over a few lines in its tile.".to_owned()),
            target: target.map(str::to_owned),
            drawn,
        }
    }

    fn card(name: &str, structure: Structure) -> Card {
        Card {
            name: name.to_owned(),
            kind: "struct · repr(Rust)".to_owned(),
            description: Some("A card.".to_owned()),
            structure,
        }
    }

    fn variant(name: &str, tiles: Vec<Tile>) -> VariantTile {
        VariantTile {
            name: name.to_owned(),
            case: None,
            tiles,
        }
    }

    /// Incoming lines from every listed form, with the focus in each form that
    /// draws its own lines a different way: from enum variants above and at
    /// the bottom, from a struct's row, and from a union's fields.
    fn scenes() -> Vec<TypeScene> {
        let incoming = [
            card(
                "Report",
                Structure::Row(vec![
                    tile("author", Some("Member"), true),
                    tile("revision", None, false),
                    // A relationship the caller does not draw keeps its target but no line.
                    tile("reviewer", Some("Member"), false),
                    tile("assignee", Some("Member"), true),
                ]),
            ),
            card(
                "Approval",
                Structure::Variants(vec![
                    variant("Open", Vec::new()),
                    variant(
                        "Named",
                        vec![
                            tile("0", Some("Member"), true),
                            tile("1", Some("Member"), true),
                        ],
                    ),
                    variant("Quorum", vec![tile("voters", Some("Member"), true)]),
                    variant("Closed", Vec::new()),
                ]),
            ),
            card(
                "Lookup",
                Structure::Stack(vec![
                    tile("id", Some("Member"), true),
                    tile("raw", None, false),
                ]),
            ),
        ];
        let own = || {
            [
                tile("parent", Some("Member"), true),
                tile("mentor", Some("Member"), true),
                tile("children", Some("Member"), true),
            ]
        };
        let [parent, mentor, children] = own();
        [
            Structure::Variants(vec![
                variant("Nested", vec![parent, mentor]),
                variant("Root", vec![children]),
            ]),
            Structure::Row(own().into()),
            Structure::Stack(own().into()),
        ]
        .into_iter()
        .map(|structure| layout(&[&incoming], &card("Member", structure), None))
        .collect()
    }

    #[test]
    fn routes_neither_cross_nor_enter_any_card() {
        for scene in scenes() {
            assert_eq!(scene.routes.len(), 9);
            routes_neither_cross_nor_enter(&scene);
        }
    }

    /// Twelve incoming cards with two lines each, too wide for one row.
    fn wide_scene() -> TypeScene {
        let incoming: Vec<Card> = (0..12)
            .map(|index| {
                card(
                    &format!("Source{index}"),
                    Structure::Row(vec![
                        tile("first", Some("Member"), true),
                        tile("second", Some("Member"), true),
                    ]),
                )
            })
            .collect();
        let focus = card("Member", Structure::Row(vec![tile("name", None, false)]));
        layout(&[&incoming], &focus, None)
    }

    #[test]
    fn incoming_cards_wrap_into_rows_in_source_order() {
        let scene = wide_scene();
        let incoming = &scene.cards[..12];
        assert!(incoming.iter().any(|card| card.y > incoming[0].y));
        // Row by row, and from left to right within a row.
        assert!(
            incoming
                .windows(2)
                .all(|pair| (pair[0].y, pair[0].x) < (pair[1].y, pair[1].x))
        );
        let bounds =
            |card: &super::CardShape| (card.x, card.y, card.x + card.width, card.y + card.height);
        for (index, card) in scene.cards.iter().enumerate() {
            assert!(card.x + card.width <= MARGIN + ROW_WIDTH || index == 12);
            for other in &scene.cards[index + 1..] {
                assert!(!overlaps(bounds(card), bounds(other)));
            }
        }
        assert_eq!(scene.routes.len(), 24);
        routes_neither_cross_nor_enter(&scene);
        routes_avoid_every_text(&scene);
        routes_leave_their_own_cards(&scene);
    }

    /// The rows the first `count` cards of `scene` take.
    fn rows(scene: &TypeScene, count: usize) -> usize {
        let mut tops: Vec<i32> = scene.cards[..count].iter().map(|card| card.y).collect();
        tops.dedup();
        tops.len()
    }

    #[test]
    fn each_group_of_incoming_cards_starts_a_row() {
        let source = |name: &str| card(name, Structure::Row(vec![tile("m", Some("Member"), true)]));
        let (first, second) = ([source("Library")], [source("Test"), source("Example")]);
        let focus = card("Member", Structure::Row(vec![tile("name", None, false)]));
        let scene = layout(&[&first, &second], &focus, None);
        assert_eq!(rows(&scene, 3), 2);
        // The second group is a row of its own, aligned to the same columns.
        assert_eq!(scene.cards[0].x, scene.cards[1].x);
        assert_eq!(scene.cards[1].y, scene.cards[2].y);
        routes_neither_cross_nor_enter(&scene);
    }

    #[test]
    fn a_focus_wider_than_a_screen_makes_room_for_more_columns() {
        let incoming: Vec<Card> = (0..16)
            .map(|index| {
                card(
                    &format!("Source{index}"),
                    Structure::Row(vec![tile("m", Some("Member"), true)]),
                )
            })
            .collect();
        let focus = |fields: usize| {
            let tiles = (0..fields)
                .map(|index| tile(&format!("field{index}"), None, false))
                .collect();
            card("Member", Structure::Row(tiles))
        };
        let narrow = layout(&[&incoming], &focus(1), None);
        let wide = layout(&[&incoming], &focus(14), None);
        assert!(wide.cards[16].width > 1800);
        assert!(rows(&wide, 16) < rows(&narrow, 16));
        routes_neither_cross_nor_enter(&wide);
    }

    #[test]
    fn a_row_holds_the_given_number_of_cards() {
        let incoming: Vec<Card> = (0..7)
            .map(|index| {
                card(
                    &format!("Source{index}"),
                    Structure::Row(vec![tile("m", Some("Member"), true)]),
                )
            })
            .collect();
        let focus = card("Member", Structure::Row(vec![tile("name", None, false)]));
        let scene = layout(&[&incoming], &focus, NonZeroUsize::new(3));
        assert_eq!(rows(&scene, 7), 3);
        assert!(
            scene.cards[..3]
                .iter()
                .all(|card| card.y == scene.cards[0].y)
        );
        routes_neither_cross_nor_enter(&scene);
    }

    fn routes_neither_cross_nor_enter(scene: &TypeScene) {
        for (index, route) in scene.routes.iter().enumerate() {
            for other in &scene.routes[index + 1..] {
                // Lines that end together share a bus and meet where they join it.
                let shared = route.points.last() == other.points.last();
                let meetings = if shared {
                    bundle_meetings(&route.points, &other.points)
                } else {
                    Vec::new()
                };
                assert!(
                    compatible(&route.points, &other.points, shared, &meetings),
                    "{:?} crosses {:?}",
                    route.points,
                    other.points
                );
            }
            for card in &scene.cards {
                let bounds = (card.x, card.y, card.x + card.width, card.y + card.height);
                // Where a segment is inside the card, it runs along a groove.
                let in_groove = |point: Point| {
                    let point = Point {
                        x: point.x.clamp(bounds.0, bounds.2),
                        y: point.y.clamp(bounds.1, bounds.3),
                    };
                    card.grooves.iter().any(|&groove| holds(groove, point))
                };
                for segment in route.points.windows(2) {
                    assert!(
                        !enters(segment[0], segment[1], bounds)
                            || in_groove(segment[0]) && in_groove(segment[1]),
                        "{:?} enters {}",
                        route.points,
                        card.name
                    );
                }
            }
        }
    }

    #[test]
    fn routes_avoid_every_text_including_their_own_cards() {
        for scene in scenes() {
            routes_avoid_every_text(&scene);
        }
    }

    fn routes_avoid_every_text(scene: &TypeScene) {
        for card in &scene.cards {
            for block in &card.texts {
                let width = block
                    .lines
                    .iter()
                    .map(|line| block.text.width(line))
                    .max()
                    .unwrap();
                let left = if block.text.right_aligned() {
                    block.x - width
                } else {
                    block.x
                };
                let bottom = block.top + block.text.height(&block.lines);
                for route in &scene.routes {
                    for segment in route.points.windows(2) {
                        assert!(
                            !enters(
                                segment[0],
                                segment[1],
                                (left, block.top, left + width, bottom)
                            ),
                            "{:?} crosses {:?} in {}",
                            route.points,
                            block.lines.first().map(AsRef::<str>::as_ref),
                            card.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn routes_leave_their_own_cards_and_cards_keep_source_order() {
        for scene in scenes() {
            routes_leave_their_own_cards(&scene);
        }
        let scene = &scenes()[0];
        let names: Vec<&str> = scene.cards.iter().map(|card| card.name.as_str()).collect();
        assert_eq!(names, ["Report", "Approval", "Lookup", "Member"]);
        assert!(
            scene.cards[..3]
                .windows(2)
                .all(|pair| pair[0].x < pair[1].x)
        );
    }

    fn routes_leave_their_own_cards(scene: &TypeScene) {
        for route in &scene.routes {
            let card = &scene.cards[route.card];
            let start = route.points[0];
            assert!(
                // A line may start on its card's right side or bottom edge.
                start.x > card.x
                    && start.x <= card.x + card.width
                    && start.y > card.y
                    && start.y <= card.y + card.height,
                "{:?} does not start in {}",
                route.points,
                card.name
            );
        }
    }

    #[test]
    fn every_route_ends_on_the_focus_top_edge() {
        for scene in scenes() {
            let focus = scene.cards.last().unwrap();
            for route in &scene.routes {
                let end = route.points.last().unwrap();
                assert_eq!(end.y, focus.y, "{:?}", route.points);
                assert!(end.x > focus.x && end.x < focus.x + focus.width);
            }
        }
    }
}
