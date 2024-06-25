mod gc;
mod openrpc_diff;

use anyhow::Context as _;
use clap::Parser;
use itertools::Itertools as _;
use openrpc_types::{resolve_within, OpenRPC};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
enum Args {
    #[command(subcommand)]
    Openrpc(Openrpc),
}

/// Subommands related to processing OpenRPC documents.
#[derive(Parser)]
enum Openrpc {
    /// Print the following to stderr:
    /// - duplicate method names
    /// - duplicate parameter names
    /// - bad optional parameters
    ///
    /// Does not validate anything else, including:
    /// - that example pairings match schemas
    /// - that Example::value and Example::externalValue are mutually exclusive
    /// - dead $refs, or JSON Schema $refs
    /// - links, runtime expressions
    /// - component keys are idents
    /// - error codes are unique
    ReportErrors { path: PathBuf },
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
    let Args::Openrpc(args) = Args::parse();
    match args {
        Openrpc::ReportErrors { path } => {
            let document = load_json::<OpenRPC>(path)?;
            let methods = resolve_within(document)?.methods;
            if let Ok(dups) = nunny::Vec::new(
                methods
                    .iter()
                    .map(|it| it.name.as_str())
                    .duplicates()
                    .collect(),
            ) {
                eprintln!(
                    "the following method names are duplicated: {}",
                    dups.join(", ")
                )
            };

            for method in &methods {
                if let Ok(dups) = nunny::Vec::new(
                    method
                        .params
                        .iter()
                        .map(|it| it.name.as_str())
                        .duplicates()
                        .collect(),
                ) {
                    eprintln!(
                        "the following parameter names on method {} are duplicated: {}",
                        method.name,
                        dups.join(", ")
                    )
                }
                if let Some((ix, name)) = method.params.iter().enumerate().find_map(|(ix, it)| {
                    (!it.required.unwrap_or_default()).then_some((ix, it.name.as_str()))
                }) {
                    if let Ok(after) = nunny::Vec::new(
                        method.params[ix..]
                            .iter()
                            .filter(|it| it.required.unwrap_or_default())
                            .map(|it| it.name.as_str())
                            .collect(),
                    ) {
                        eprintln!("the following required parameters on method {} follow the optional parameter {}: {}", method.name, name, after.join(", "))
                    }
                }
            }

            Ok(())
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
    imp::<T>(path.as_ref())
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
