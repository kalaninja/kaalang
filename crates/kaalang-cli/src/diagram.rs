use std::{
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use clap::{ArgGroup, builder::NonEmptyStringValueParser};
use kaalang_svg::RenderError;

use crate::{datatype, workspace};

#[derive(clap::Args)]
#[command(group(ArgGroup::new("item").required(true).args(["flow", "type"])))]
pub(super) struct Diagram {
    /// Package to take the flow or type from
    #[arg(short)]
    package: Option<String>,
    /// Rust path of the flow from its crate root
    #[arg(long, value_parser = NonEmptyStringValueParser::new())]
    flow: Option<String>,
    /// Rust path of the type from its crate root
    #[arg(long = "type", id = "type", value_parser = NonEmptyStringValueParser::new())]
    ty: Option<String>,
    #[arg(
        short,
        help = "Output path [default: ./NAME.svg after the path's last segment, or ./NAME_collapsed.svg with --collapse-cycles]"
    )]
    output: Option<PathBuf>,
    /// Draw each cycle as one described node
    #[arg(long, conflicts_with = "type")]
    collapse_cycles: bool,
    /// Incoming types to a row [default: as many as fit a Full HD screen]
    #[arg(long, conflicts_with = "flow")]
    columns: Option<NonZeroUsize>,
}

impl Diagram {
    /// Renders the flow or type and prints the written path.
    pub(super) fn run(self) -> Result<(), String> {
        let (path, svg, sources) = match (&self.flow, &self.ty) {
            (Some(flow), _) => {
                let (svg, file) = self.flow(flow)?;
                (flow, svg, vec![file])
            }
            (None, Some(ty)) => {
                let options = kaalang_svg::TypeOptions {
                    columns: self.columns,
                };
                let drawn = datatype::draw(ty, self.package.as_deref(), options)?;
                (ty, drawn.svg, drawn.files)
            }
            (None, None) => unreachable!("clap requires --flow or --type"),
        };
        let name = path
            .rsplit_once("::")
            .map_or(path.as_str(), |(_, name)| name);
        let name = name.strip_prefix("r#").unwrap_or(name);
        let output = self.output.unwrap_or_else(|| {
            PathBuf::from(if self.collapse_cycles {
                format!("{name}_collapsed.svg")
            } else {
                format!("{name}.svg")
            })
        });
        for source in &sources {
            if overwrites(source, &output)? {
                return Err("the output path would overwrite the source file".to_owned());
            }
        }
        // Rendering finished in memory, so a failed render never touches the output.
        fs::write(&output, &svg)
            .map_err(|error| format!("could not write `{}`: {error}", output.display()))?;

        println!("{}", output.display());
        Ok(())
    }

    /// Renders the flow at `path` and returns it with the file declaring it.
    fn flow(&self, path: &str) -> Result<(String, PathBuf), String> {
        let found = workspace::find(path, self.package.as_deref())?;
        let svg = kaalang_svg::render_flow(
            &found.source,
            found.position,
            kaalang_svg::RenderOptions {
                collapse_cycles: self.collapse_cycles,
            },
        )
        .map_err(|mut error| {
            // The renderer knows only the path within the file.
            if let RenderError::InvalidFlow { name, .. }
            | RenderError::UnroutableTopology { name, .. } = &mut error
            {
                path.clone_into(name);
            }
            format!("{}: {error}", found.file.display())
        })?;
        Ok((svg, found.file))
    }
}

/// Whether writing `output` would replace `source`, even through a link.
fn overwrites(source: &Path, output: &Path) -> Result<bool, String> {
    match same_file::is_same_file(source, output) {
        Ok(same) => Ok(same),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "could not check output `{}`: {error}",
            output.display()
        )),
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

    #[test]
    fn takes_exactly_one_of_flow_and_type() {
        for line in [
            "cargo-kaalang diagram",
            "cargo-kaalang diagram --flow decide --type Member",
            "cargo-kaalang diagram --type=",
        ] {
            assert!(
                Cli::try_parse_from(line.split_whitespace()).is_err(),
                "{line}"
            );
        }
        assert!(
            Cli::try_parse_from("cargo-kaalang diagram --type Member".split_whitespace()).is_ok()
        );
    }

    #[test]
    fn takes_a_positive_column_count_with_a_type_only() {
        for line in [
            "cargo-kaalang diagram --type Member --columns 0",
            "cargo-kaalang diagram --flow decide --columns 2",
        ] {
            assert!(
                Cli::try_parse_from(line.split_whitespace()).is_err(),
                "{line}"
            );
        }
        let line = "cargo-kaalang diagram --type Member --columns 3";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_ok());
    }

    #[test]
    fn rejects_collapse_cycles_with_a_type() {
        let line = "cargo-kaalang diagram --type Member --collapse-cycles";
        assert!(Cli::try_parse_from(line.split_whitespace()).is_err());
    }
}
