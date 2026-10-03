mod diagram;

use std::{env, ffi::OsString, process::ExitCode};

use clap::{Parser, Subcommand};

use crate::diagram::Diagram;

/// The binary's name from its `[[bin]]` entry.
const BIN_NAME: &str = env!("CARGO_BIN_NAME");

#[derive(Parser)]
#[command(name = BIN_NAME, bin_name = "cargo kaalang", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render one flow as an SVG file
    Diagram(Diagram),
}

fn main() -> ExitCode {
    let cli = Cli::parse_from(without_subcommand_name(env::args_os()));
    let result = match cli.command {
        Command::Diagram(diagram) => diagram.run(),
    };
    if let Err(error) = result {
        eprintln!("{BIN_NAME}: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Drops the `kaalang` word cargo passes after the program path.
fn without_subcommand_name(
    arguments: impl IntoIterator<Item = OsString>,
) -> impl Iterator<Item = OsString> {
    let mut arguments = arguments.into_iter().peekable();
    let program = arguments.next();
    arguments.next_if(|argument| argument == "kaalang");
    program.into_iter().chain(arguments)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn command_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn accepts_cargo_and_direct_invocation_shapes() {
        for line in [
            "cargo-kaalang diagram flow.rs --flow decide",
            "cargo-kaalang kaalang diagram flow.rs --flow decide",
        ] {
            let arguments = without_subcommand_name(line.split_whitespace().map(OsString::from));
            Cli::try_parse_from(arguments).unwrap();
        }
    }
}
