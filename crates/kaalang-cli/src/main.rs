use std::{env, ffi::OsString, fs, path::PathBuf, process::ExitCode};

const USAGE: &str =
    "usage: cargo kaalang diagram <source.rs> --flow <name> [--collapse-cycles] [-o <path>]";

fn main() -> ExitCode {
    match run(env::args_os()) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cargo-kaalang: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Returns what to print on success: the written path, usage, or version.
fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<String, String> {
    let options = match parse_arguments(arguments)? {
        Command::Diagram(options) => options,
        Command::Help => return Ok(USAGE.to_owned()),
        Command::Version => return Ok(format!("cargo-kaalang {}", env!("CARGO_PKG_VERSION"))),
    };
    let source = fs::read_to_string(&options.source)
        .map_err(|error| format!("could not read `{}`: {error}", options.source.display()))?;
    let svg = kaalang_svg::render_source_with_options(
        &source,
        &options.flow,
        kaalang_svg::RenderOptions {
            collapse_cycles: options.collapse_cycles,
        },
    )
    .map_err(|error| error.to_string())?;
    let output = options.output.unwrap_or_else(|| {
        PathBuf::from(if options.collapse_cycles {
            format!("{}_collapsed.svg", options.flow)
        } else {
            format!("{}.svg", options.flow)
        })
    });
    let same_source = match same_file::is_same_file(&options.source, &output) {
        Ok(same) => same,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!(
                "could not check output `{}`: {error}",
                output.display()
            ));
        }
    };
    if same_source {
        return Err("the output path would overwrite the source file".to_owned());
    }
    // Rendering finished in memory, so a failed flow never touches the output.
    fs::write(&output, &svg)
        .map_err(|error| format!("could not write `{}`: {error}", output.display()))?;

    Ok(output.display().to_string())
}

enum Command {
    Diagram(Options),
    Help,
    Version,
}

struct Options {
    source: PathBuf,
    flow: String,
    output: Option<PathBuf>,
    collapse_cycles: bool,
}

fn parse_arguments(arguments: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    // Skip the program name, then the `kaalang` word cargo inserts.
    let mut arguments = arguments.into_iter().skip(1).peekable();
    arguments.next_if(|argument| argument == "kaalang");
    match arguments
        .next()
        .as_ref()
        .and_then(|argument| argument.to_str())
    {
        Some("diagram") => {}
        Some("-h" | "--help") => return Ok(Command::Help),
        Some("-V" | "--version") => return Ok(Command::Version),
        _ => return Err(USAGE.to_owned()),
    }
    let source = arguments
        .next()
        .filter(|argument| !argument.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| USAGE.to_owned())?;
    let mut flow = None;
    let mut output = None;
    let mut collapse_cycles = false;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--flow") if flow.is_none() => {
                flow = Some(
                    arguments
                        .next()
                        .and_then(|value| value.into_string().ok())
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| "`--flow` requires a UTF-8 flow name".to_owned())?,
                );
            }
            Some("-o") if output.is_none() => {
                output = Some(
                    arguments
                        .next()
                        .map(PathBuf::from)
                        .ok_or_else(|| "`-o` requires an output path".to_owned())?,
                );
            }
            Some("--collapse-cycles") if !collapse_cycles => collapse_cycles = true,
            _ => return Err(USAGE.to_owned()),
        }
    }
    let flow = flow.ok_or_else(|| "missing required `--flow <name>`\n\n".to_owned() + USAGE)?;

    Ok(Command::Diagram(Options {
        source,
        flow,
        output,
        collapse_cycles,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Result<Options, String> {
        match parse_arguments(line.split_whitespace().map(OsString::from))? {
            Command::Diagram(options) => Ok(options),
            Command::Help | Command::Version => Err("not a diagram command".to_owned()),
        }
    }

    #[test]
    fn accepts_cargo_and_direct_invocation_shapes() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide",
            "cargo-kaalang kaalang diagram flow.rs --flow decide",
        ] {
            let options = parse(line).unwrap();
            assert_eq!(options.source, PathBuf::from("flow.rs"));
            assert_eq!(options.flow, "decide");
            assert!(!options.collapse_cycles);
        }
    }

    #[test]
    fn accepts_collapse_cycles_anywhere_among_the_options() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide --collapse-cycles",
            "cargo-kaalang diagram flow.rs --collapse-cycles -o chosen.svg --flow decide",
        ] {
            assert!(parse(line).unwrap().collapse_cycles);
        }
    }

    #[test]
    fn rejects_unknown_or_duplicate_collapse_options() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide --collapsed",
            "cargo-kaalang diagram flow.rs --flow decide --collapse-cycles --collapse-cycles",
        ] {
            assert!(parse(line).is_err());
        }
    }
}
