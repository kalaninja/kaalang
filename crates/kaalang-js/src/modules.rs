//! The compiler's module file candidates in the form the editor host reads.

use kaalang_compiler::SourceFile;
use serde_json::json;

use crate::utf16;

pub(crate) fn items(
    text: &str,
    cfg: &str,
    stem: &str,
    owns_directory: bool,
) -> Result<String, String> {
    let source = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
    let offsets = utf16::Offsets::new(text);
    let (modules, types) = source.module_items(stem, owns_directory);
    let modules: Vec<_> = modules.into_iter().map(|declaration| {
        let files: Vec<_> = declaration.files.into_iter().map(|file| {
            json!({"parts": file.parts, "ownsDirectory": file.owns_directory})
        }).collect();
        json!({"module": declaration.module, "offset": offsets.at(declaration.offset), "files": files})
    }).collect();
    let types: Vec<_> = types.into_iter().map(|byte| offsets.at(byte)).collect();
    let bodies: Vec<_> = source
        .module_bodies()
        .into_iter()
        .map(|range| [offsets.at(range.start), offsets.at(range.end)])
        .collect();
    Ok(json!({"modules": modules, "types": types, "bodies": bodies}).to_string())
}
