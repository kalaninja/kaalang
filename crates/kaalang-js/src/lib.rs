//! JavaScript bindings for kaalang, compiled to WebAssembly.
//!
//! Every offset and range that crosses to JavaScript counts UTF-16 code units
//! of the text as given, as JavaScript strings and the editors' APIs do; the
//! column an error's message gives counts characters. A call
//! answers in JSON, a string JavaScript owns, and the module forgets the text
//! it parsed. A host can retain a workspace index of resolved relationships
//! across queries, until its source snapshot changes.

use wasm_bindgen::{JsError, prelude::wasm_bindgen};

mod declared;
mod draw;
#[cfg(test)]
mod fixture;
mod modules;
mod types;
mod utf16;
mod workspace;

/// Draws the flow at `position` among those that the target `cfg` compiles in
/// `text`, with each cycle as one node when `collapse_cycles` is set, as JSON:
/// `{"svg": …, "hotspots": [{"kind", "left", "top", "right", "bottom",
/// "start", "end", for a call "calleeStart", "calleeEnd", and for a connection
/// or a merge "ribbon", its positions in the answer's "wires", instead of
/// "start" and "end"}], "wires": [{"name", "start", "end"}]}`. `kind` is
/// `node`, `call`, `wire`, `connection`, or `merge`; a rectangle is in the
/// SVG's user units, and a wire's range is its binding. `cfg` is the target's
/// cfg set as `rustc --print cfg` prints it, empty for none. The answer is
/// `null` when the selected flow is absent. When given, `path` selects the
/// flow previously returned by [`declared_at`], even after other flows move.
///
/// # Errors
///
/// Throws why the flow cannot be drawn: `cfg` or `text` does not parse, or the
/// flow is not valid.
#[wasm_bindgen(js_name = drawFlow)]
pub fn draw_flow(
    text: &str,
    cfg: &str,
    position: usize,
    collapse_cycles: bool,
    path: Option<String>,
) -> Result<String, JsError> {
    answer(draw::flow(text, cfg, position, collapse_cycles, path))
}

/// Draws the type at `path` within `text`, read for the target `cfg`, as the
/// focus of a type diagram, under the types `context` finds referring to it,
/// as JSON: `{"svg": …, "hotspots": [{"kind", "left", "top", "right",
/// "bottom", "source", "start", "end"}], "cards": [{"source", "path",
/// "start", "end"}]}`. `kind` is `path` for a path a tile's type names,
/// `target` for a relationship's target, and `header` for an incoming card's
/// header; its range is the path's last segment, the target's type, or the
/// card's type's name. `cards` lists the focus, then each incoming card, by
/// its path within its text and the range of its name. A `source` is 0 for
/// `text` and n for the nth file of `context`, whose ranges count its own text.
/// A path's `fieldText` gives its field-type line and UTF-16 range on that
/// line. `otherPlaces`, when present, lists other readings of the same cards
/// whose Cargo positions the editor can still supply.
///
/// `context` is JSON, empty for none. The focus and each entry of `files`
/// carry the editor's reference `offsets`, hover `modules`, and `places` by
/// type path within the file. Each file also carries its `text` and `cfg`.
/// Readings of a shared file under different cfgs use the same `uri`.
/// A place gives the Cargo `package`, target `group` (0–3), `crate` name and
/// `root` path, the crate-relative `module`, and `order` as source offsets
/// from the root through module declarations to the type. These determine
/// card order, rows and headers. An empty list excludes a type traced outside
/// the focus's workspace targets. With only hover information, a header uses
/// its module path. An unreadable incoming file draws no card. The answer is
/// `null` when no type has `path`.
///
/// # Errors
///
/// Throws why the type cannot be drawn: `cfg`, `text`, or `context` does not
/// parse, or the type is not valid.
#[wasm_bindgen(js_name = drawType)]
pub fn draw_type(text: &str, cfg: &str, path: &str, context: &str) -> Result<String, JsError> {
    answer(types::ty(text, cfg, path, context))
}

/// The UTF-16 start of the final identifier in each active relationship target
/// in `text`, as a JSON array. Editors can request a definition at each offset
/// when their reference provider omits an incoming type behind a re-export.
///
/// # Errors
///
/// Throws when `cfg` or `text` does not parse.
#[wasm_bindgen(js_name = relationshipTargets)]
pub fn relationship_targets(text: &str, cfg: &str) -> Result<String, JsError> {
    answer(types::relationship_targets(text, cfg))
}

/// The active types' name offsets and the external modules this file declares
/// for `cfg`. Each module has a path, its name's UTF-16 offset, and the
/// candidate files to try relative to the containing directory.
/// `bodies` lists UTF-16 ranges inside the braces of functions, impls and
/// traits whose contents do not affect workspace name resolution.
/// `owns_directory` holds for a crate root, `mod.rs`, or an explicit `#[path]`.
///
/// # Errors
///
/// Throws when `cfg` or `text` does not parse.
#[wasm_bindgen(js_name = moduleItems)]
pub fn module_items(
    text: &str,
    cfg: &str,
    file_stem: &str,
    owns_directory: bool,
) -> Result<String, JsError> {
    answer(modules::items(text, cfg, file_stem, owns_directory))
}

/// What `text`, read for the target `cfg`, declares at `offset`, in UTF-16
/// code units, that kaalang draws, as JSON: `{"flow": position, "path": path}`,
/// `{"type": path}`, or `null`, as for an offset past the end of `text`.
///
/// # Errors
///
/// Throws why the text cannot be read: `cfg` or `text` does not parse.
#[wasm_bindgen(js_name = declaredAt)]
pub fn declared_at(text: &str, cfg: &str, offset: u32) -> Result<String, JsError> {
    answer(declared::at(text, cfg, offset))
}

/// The active declarations in source order as `[firstCaret, lastCaret, declaration]`
/// triples in UTF-16. Where two declarations meet, the later one owns the caret.
///
/// # Errors
///
/// Throws when `cfg` or `text` does not parse.
#[wasm_bindgen(js_name = declaredRanges)]
pub fn declared_ranges(text: &str, cfg: &str) -> Result<String, JsError> {
    answer(declared::ranges(text, cfg))
}

/// No span outlives a call. Release its source text before returning to the worker.
fn answer<T>(result: Result<T, String>) -> Result<T, JsError> {
    proc_macro2::extra::invalidate_current_thread_spans();
    result.map_err(|message| JsError::new(&message))
}

/// Writes a panic's message to `console.error` before the module traps, since
/// the trap itself says only "unreachable". Instantiating the module runs it.
#[wasm_bindgen(start)]
fn report_panics() {
    std::panic::set_hook(Box::new(|info| console_error(&info.to_string())));
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}
