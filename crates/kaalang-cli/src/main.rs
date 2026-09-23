use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::PathBuf,
    process::ExitCode,
};

const USAGE: &str =
    "usage: cargo kaalang diagram <source.rs> --flow <name> [--collapse-loops] [-o <path>]";

fn main() -> ExitCode {
    match run(env::args_os()) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cargo-kaalang: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<PathBuf, String> {
    let options = parse_arguments(arguments)?;
    let source = fs::read_to_string(&options.source)
        .map_err(|error| format!("could not read `{}`: {error}", options.source.display()))?;
    let svg = kaalang_svg::render_source_with_options(
        &source,
        &options.flow,
        kaalang_svg::RenderOptions {
            collapse_loops: options.collapse_loops,
        },
    )
    .map_err(|error| error.to_string())?;
    let output = options.output.unwrap_or_else(|| {
        PathBuf::from(if options.collapse_loops {
            format!("{}_collapsed.svg", options.flow)
        } else {
            format!("{}.svg", options.flow)
        })
    });
    // Rendering finished in memory, so a failed flow never touches the output.
    fs::write(&output, &svg)
        .map_err(|error| format!("could not write `{}`: {error}", output.display()))?;

    Ok(output)
}

struct Options {
    source: PathBuf,
    flow: String,
    output: Option<PathBuf>,
    collapse_loops: bool,
}

fn parse_arguments(arguments: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    // Skip the program name, then the `kaalang` word cargo inserts.
    let mut arguments = arguments.into_iter().skip(1).peekable();
    arguments.next_if(|argument| argument == "kaalang");
    if arguments.next().as_deref() != Some(OsStr::new("diagram")) {
        return Err(USAGE.to_owned());
    }
    let source = arguments
        .next()
        .filter(|argument| !argument.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| USAGE.to_owned())?;
    let mut flow = None;
    let mut output = None;
    let mut collapse_loops = false;
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
            Some("--collapse-loops") if !collapse_loops => collapse_loops = true,
            _ => return Err(USAGE.to_owned()),
        }
    }
    let flow = flow.ok_or_else(|| "missing required `--flow <name>`\n\n".to_owned() + USAGE)?;

    Ok(Options {
        source,
        flow,
        output,
        collapse_loops,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Result<Options, String> {
        parse_arguments(line.split_whitespace().map(OsString::from))
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
            assert!(!options.collapse_loops);
        }
    }

    #[test]
    fn accepts_collapsed_output_with_or_without_an_explicit_path() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide --collapse-loops",
            "cargo-kaalang diagram flow.rs --collapse-loops -o chosen.svg --flow decide",
        ] {
            assert!(parse(line).unwrap().collapse_loops);
        }
    }

    #[test]
    fn rejects_unknown_or_duplicate_collapse_options() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide --collapsed",
            "cargo-kaalang diagram flow.rs --flow decide --collapse-loops --collapse-loops",
        ] {
            assert!(parse(line).is_err());
        }
    }
}
