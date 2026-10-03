use std::{fs, path::PathBuf};

use clap::builder::NonEmptyStringValueParser;

#[derive(clap::Args)]
pub(super) struct Diagram {
    /// Rust source file that contains the flow
    source: PathBuf,
    /// Name of the flow to render
    #[arg(long, value_parser = NonEmptyStringValueParser::new())]
    flow: String,
    #[arg(
        short,
        help = "Output path [default: ./FLOW.svg, or ./FLOW_collapsed.svg with --collapse-cycles]"
    )]
    output: Option<PathBuf>,
    /// Draw each cycle as one described node
    #[arg(long)]
    collapse_cycles: bool,
}

impl Diagram {
    /// Renders the flow and prints the written path.
    pub(super) fn run(self) -> Result<(), String> {
        let source = fs::read_to_string(&self.source)
            .map_err(|error| format!("could not read `{}`: {error}", self.source.display()))?;
        let svg = kaalang_svg::render_source_with_options(
            &source,
            &self.flow,
            kaalang_svg::RenderOptions {
                collapse_cycles: self.collapse_cycles,
            },
        )
        .map_err(|error| error.to_string())?;
        let output = self.output.unwrap_or_else(|| {
            PathBuf::from(if self.collapse_cycles {
                format!("{}_collapsed.svg", self.flow)
            } else {
                format!("{}.svg", self.flow)
            })
        });
        let same_source = match same_file::is_same_file(&self.source, &output) {
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

        println!("{}", output.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::Cli;

    #[test]
    fn rejects_a_repeated_collapse_cycles_flag() {
        let line =
            "cargo-kaalang diagram flow.rs --flow decide --collapse-cycles --collapse-cycles";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_err());
    }

    #[test]
    fn rejects_an_empty_flow_name() {
        let line = "cargo-kaalang diagram flow.rs --flow=";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_err());
    }
}
