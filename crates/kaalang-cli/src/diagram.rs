use std::{fs, path::PathBuf};

use clap::builder::NonEmptyStringValueParser;
use kaalang_svg::RenderError;

use crate::workspace;

#[derive(clap::Args)]
pub(super) struct Diagram {
    /// Package to take the flow from
    #[arg(short)]
    package: Option<String>,
    /// Rust path of the flow from its crate root
    #[arg(long, value_parser = NonEmptyStringValueParser::new())]
    flow: String,
    #[arg(
        short,
        help = "Output path [default: ./NAME.svg after the flow path's last segment, or ./NAME_collapsed.svg with --collapse-cycles]"
    )]
    output: Option<PathBuf>,
    /// Draw each cycle as one described node
    #[arg(long)]
    collapse_cycles: bool,
}

impl Diagram {
    /// Renders the flow and prints the written path.
    pub(super) fn run(self) -> Result<(), String> {
        let found = workspace::find(&self.flow, self.package.as_deref())?;
        let svg = kaalang_svg::render_source_with_options(
            &found.source,
            &found.path_in_file,
            kaalang_svg::RenderOptions {
                collapse_cycles: self.collapse_cycles,
            },
        )
        .map_err(|mut error| {
            // The renderer knows only the path within the file.
            if let RenderError::InvalidFlow { name, .. }
            | RenderError::UnroutableTopology { name, .. } = &mut error
            {
                name.clone_from(&self.flow);
            }
            format!("{}: {error}", found.file.display())
        })?;
        let name = self
            .flow
            .rsplit_once("::")
            .map_or(self.flow.as_str(), |(_, name)| name);
        let name = name.strip_prefix("r#").unwrap_or(name);
        let output = self.output.unwrap_or_else(|| {
            PathBuf::from(if self.collapse_cycles {
                format!("{name}_collapsed.svg")
            } else {
                format!("{name}.svg")
            })
        });
        let same_source = match same_file::is_same_file(&found.file, &output) {
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
        let line = "cargo-kaalang diagram --flow decide --collapse-cycles --collapse-cycles";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_err());
    }

    #[test]
    fn rejects_an_empty_flow_name() {
        let line = "cargo-kaalang diagram --flow=";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_err());
    }
}
