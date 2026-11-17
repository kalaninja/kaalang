//! JavaScript bindings for kaalang, compiled to WebAssembly.
//!
//! Every offset and range that crosses to JavaScript counts UTF-16 code units
//! of the text as given, as JavaScript strings and the editors' APIs do; the
//! column an error's message gives counts characters. A call
//! answers in JSON, a string JavaScript owns, and the module forgets the text
//! it parsed: nothing of a call stays in its memory, however often a page
//! draws.

use wasm_bindgen::{JsError, prelude::wasm_bindgen};

mod declared;
mod draw;
#[cfg(test)]
mod fixture;
mod utf16;

/// Draws the flow at `position` among those that the target `cfg` compiles in
/// `text`, with each cycle as one node when `collapse_cycles` is set, as JSON:
/// `{"svg": …, "hotspots": [{"kind", "left", "top", "right", "bottom",
/// "start", "end", and for a call "calleeStart", "calleeEnd"}]}`. `kind` is
/// `node`, `call`, or `wire`; a rectangle is in the SVG's user units. `cfg` is
/// the target's cfg set as `rustc --print cfg` prints it, empty for none.
///
/// # Errors
///
/// Throws why the flow cannot be drawn: `cfg` or `text` does not parse, no flow
/// stands at `position`, or the flow is not valid.
#[wasm_bindgen(js_name = drawFlow)]
pub fn draw_flow(
    text: &str,
    cfg: &str,
    position: usize,
    collapse_cycles: bool,
) -> Result<String, JsError> {
    let drawn = draw::flow(text, cfg, position, collapse_cycles);
    forget_parsed_text();
    drawn.map_err(|message| JsError::new(&message))
}

/// What `text`, read for the target `cfg`, declares at `offset`, in UTF-16
/// code units, that kaalang draws, as JSON: `{"flow": position}`,
/// `{"type": path}`, or `null`, as for an offset past the end of `text`.
///
/// # Errors
///
/// Throws why the text cannot be read: `cfg` or `text` does not parse.
#[wasm_bindgen(js_name = declaredAt)]
pub fn declared_at(text: &str, cfg: &str, offset: u32) -> Result<String, JsError> {
    let declared = declared::at(text, cfg, offset);
    forget_parsed_text();
    declared.map_err(|message| JsError::new(&message))
}

/// Drops every source text proc-macro2 still holds for spans. Nothing parsed
/// outlives a call, so none of those spans is used again.
fn forget_parsed_text() {
    proc_macro2::extra::invalidate_current_thread_spans();
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
