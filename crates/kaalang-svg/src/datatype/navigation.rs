//! The hotspots of a type diagram: each path a tile's type names, each
//! relationship's target, and each incoming card's header, with the source
//! each stands for.

use super::{
    Card,
    layout::{Bounds, Text, TextBlock, TypeScene},
};
use crate::{
    Hotspot, HotspotKind,
    text::{RichText, Style, block_metrics, code_ranges, text_width},
};

/// The hotspots of `scene`, drawn for `focus` under `incoming`, the cards in
/// the caller's order. A card counts as 0 for the focus, and from 1 on for
/// the incoming cards in that order.
pub(super) fn collect(scene: &TypeScene, focus: &Card, incoming: &[&Card]) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();
    for (index, shape) in scene.cards.iter().enumerate() {
        let (number, card) = if index == scene.focus {
            (0, focus)
        } else {
            (index + 1, incoming[index])
        };
        let tiles = card.structure.tiles();
        let mut push = |kind, (left, top, right, bottom): Bounds, span| {
            hotspots.push(Hotspot {
                kind,
                left,
                top,
                right,
                bottom,
                span: Some(span),
            });
        };
        for block in &shape.texts {
            match (block.text, block.tile) {
                (Text::Name, _) if number > 0 => {
                    push(
                        HotspotKind::Header { card: number },
                        header(block),
                        card.name_span,
                    );
                }
                (Text::Type, Some(tile)) => {
                    let paths = &tiles[tile].paths;
                    let font = block.text.font();
                    let baselines =
                        block_metrics(&block.lines, font, block.text.line_height()).baselines;
                    code_ranges(
                        &block.lines,
                        font,
                        paths.iter().map(|path| path.range.clone()),
                        |index, line, left, right| {
                            let y = block.top + baselines[line];
                            let rect = (block.x + left, y - font, block.x + right, y + font / 2);
                            push(HotspotKind::Path { card: number }, rect, paths[index].span);
                        },
                    );
                }
                (Text::Target, Some(tile)) => {
                    if let (Some(target), Some(span)) =
                        (&tiles[tile].target, tiles[tile].target_span)
                    {
                        push(
                            HotspotKind::Target { card: number },
                            target_rect(block, target),
                            span,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    hotspots
}

/// The rectangle of a target's path, which ends its line, right-aligned.
fn target_rect(block: &TextBlock, target: &str) -> Bounds {
    let font = block.text.font();
    let baselines = block_metrics(&block.lines, font, block.text.line_height()).baselines;
    let y = block.top + baselines.last().copied().unwrap_or_default();
    let width = text_width(&RichText::code(target), font);
    (block.x - width, y - font, block.x, y + font / 2)
}

/// The rectangle of a card's name line, which its stylesheet sets in bold.
fn header(block: &TextBlock) -> Bounds {
    let font = block.text.font();
    let metrics = block_metrics(&block.lines, font, block.text.line_height());
    let bold = Style {
        bold: true,
        ..Style::default()
    };
    let width = block
        .lines
        .iter()
        .map(|line| text_width(&RichText::marked(line.as_ref(), bold), font))
        .max()
        .unwrap_or_default();
    (
        block.x,
        block.top,
        block.x + width,
        block.top + metrics.height,
    )
}
