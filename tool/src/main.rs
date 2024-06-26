mod gc;
mod openrpc_diff;

use anyhow::{bail, Context as _};
use ascii::AsciiChar;
use clap::Parser;
use itertools::Itertools as _;
use openrpc_types::resolve_within;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
enum Args {
    #[command(subcommand)]
    Openrpc(Openrpc),
    /// Interpret stdin as a `delimter`-separated series of lines, with a header,
    /// and print JSON.
    Csv2json {
        #[arg(short, long, default_value_t = Char(AsciiChar::Tab))]
        delimiter: Char,
    },
}

/// Subommands related to processing OpenRPC documents.
#[derive(Parser)]
enum Openrpc {
    /// Validates that:
    /// - method names are unique
    /// - parameter names are unique
    /// - there are no optional parameters
    ///
    /// Does not validate anything else, including:
    /// - that example pairings match schemas
    /// - that Example::value and Example::externalValue are mutually exclusive
    /// - dead $refs, or JSON Schema $refs
    /// - links, runtime expressions
    /// - component keys are idents
    /// - error codes are unique
    Validate { path: PathBuf },
    /// Print a summary of semantic differences between the `left` and `right`
    /// OpenRPC schemas.
    Diff { left: PathBuf, right: PathBuf },
    /// Interpret `select` as a table of methods to include in `openrpc`, outputting
    /// a new schema with only the selected methods.
    Select {
        openrpc: PathBuf,
        select: PathBuf,
        /// Specify a new title for the schema
        #[arg(long)]
        overwrite_title: Option<String>,
        /// Specify a new version for the schema
        #[arg(long)]
        overwrite_version: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let openrpc = match Args::parse() {
        Args::Openrpc(subcommand) => subcommand,
        Args::Csv2json {
            delimiter: Char(delimiter),
        } => {
            let mut records = csv::ReaderBuilder::new()
                .delimiter(delimiter.as_byte())
                .from_reader(io::stdin())
                .deserialize::<BTreeMap<String, String>>()
                .collect::<Result<Vec<_>, _>>()?;
            for record in &mut records {
                record.retain(|_k, v| !v.is_empty())
            }
            serde_json::to_writer_pretty(io::stdout(), &records)?;
            return Ok(());
        }
    };
    match openrpc {
        Openrpc::Validate { path } => {
            let mut errors = vec![];
            let document = resolve_within(load_json(path)?)?;
            errors.extend(
                document
                    .methods
                    .iter()
                    .map(|it| it.name.as_str())
                    .duplicates()
                    .map(|it| format!("duplicate method name {}", it)),
            );
            for method in &document.methods {
                errors.extend(
                    method
                        .params
                        .iter()
                        .map(|it| it.name.as_str())
                        .duplicates()
                        .map(|it| {
                            format!("duplicate parameter name {} on method {}", it, method.name)
                        }),
                );
                errors.extend(
                    method
                        .params
                        .iter()
                        .filter(|it| !it.required.unwrap_or_default())
                        .map(|it| {
                            format!(
                                "non-required parameters are forbidden by the FIP, \
                                 but parameter {} on method {} is not required",
                                it.name, method.name
                            )
                        }),
                )
            }

            match errors.len() {
                0 => Ok(()),
                n => {
                    for error in errors {
                        eprintln!("{}", error);
                    }
                    bail!("found {} errors", n)
                }
            }
        }
        Openrpc::Diff { left, right } => {
            let summary = openrpc_diff::diff(load_json(left)?, load_json(right)?)?;
            serde_json::to_writer_pretty(io::stdout(), &summary)?;
            Ok(())
        }
        Openrpc::Select {
            openrpc,
            select,
            overwrite_title,
            overwrite_version,
        } => {
            let mut openrpc = resolve_within(load_json(openrpc)?)?;
            let select = load_json::<Vec<Select>>(select)?
                .into_iter()
                .filter(|it| matches!(it.include, Some(InclusionDirective::Include)))
                // formatting the name like this is a hack
                .map(|it| (format!("Filecoin.{}", it.method), it.description))
                .collect::<BTreeMap<_, _>>();
            openrpc.methods.retain_mut(|it| match select.get(&it.name) {
                Some(new_description) => {
                    if new_description.is_some() && it.description.is_none() {
                        it.description.clone_from(new_description)
                    }
                    true
                }
                None => false,
            });
            gc::prune_schemas(&mut openrpc)?;
            if let Ok(missed) = nunny::Vec::new(
                select
                    .keys()
                    .collect::<BTreeSet<_>>()
                    .difference(&openrpc.methods.iter().map(|it| &it.name).collect())
                    .collect(),
            ) {
                eprintln!(
                    "the following selected methods were not present: {}",
                    missed.iter().join(", ")
                )
            }
            if let Some(title) = overwrite_title {
                openrpc.info.title = title
            }
            if let Some(version) = overwrite_version {
                openrpc.info.version = version
            }
            serde_json::to_writer_pretty(io::stdout(), &openrpc)?;
            Ok(())
        }
    }
}

fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> anyhow::Result<T> {
    fn imp<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
        Ok(serde_path_to_error::deserialize(
            &mut serde_json::Deserializer::from_reader(File::open(path)?),
        )?)
    }
    imp(path.as_ref())
        .with_context(|| format!("couldn't load json from file {}", path.as_ref().display()))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Select {
    description: Option<String>,
    include: Option<InclusionDirective>,
    method: String,
}

#[derive(Serialize, Deserialize)]
enum InclusionDirective {
    Discussion,
    Include,
    Exclude,
}

#[derive(Clone)]
struct Char(AsciiChar);

impl std::str::FromStr for Char {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(AsciiChar::from_ascii(char::from_str(s)?)?))
    }
}

impl fmt::Display for Char {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
