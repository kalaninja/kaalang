use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::PathBuf,
    process::ExitCode,
};

const USAGE: &str = "usage: cargo kaalang diagram <source.rs> --flow <name> [-o <path>]";

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
    let svg =
        kaalang_svg::render_source(&source, &options.flow).map_err(|error| error.to_string())?;
    let output = options
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}.svg", options.flow)));
    // Rendering finished in memory, so a failed flow never touches the output.
    fs::write(&output, &svg)
        .map_err(|error| format!("could not write `{}`: {error}", output.display()))?;

    Ok(output)
}

struct Options {
    source: PathBuf,
    flow: String,
    output: Option<PathBuf>,
}

fn parse_arguments(arguments: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();
    let mut arguments = arguments.peekable();
    if arguments
        .peek()
        .is_some_and(|argument| argument == "kaalang")
    {
        arguments.next();
    }
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
            _ => return Err(USAGE.to_owned()),
        }
    }
    let flow = flow.ok_or_else(|| "missing required `--flow <name>`\n\n".to_owned() + USAGE)?;

    Ok(Options {
        source,
        flow,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_cargo_and_direct_invocation_shapes() {
        for arguments in [
            vec!["cargo-kaalang", "diagram", "flow.rs", "--flow", "decide"],
            vec![
                "cargo-kaalang",
                "kaalang",
                "diagram",
                "flow.rs",
                "--flow",
                "decide",
            ],
        ] {
            let options = parse_arguments(arguments.into_iter().map(OsString::from)).unwrap();
            assert_eq!(options.source, PathBuf::from("flow.rs"));
            assert_eq!(options.flow, "decide");
        }
    }
}
