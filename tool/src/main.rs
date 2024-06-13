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

#[derive(Parser)]
enum Openrpc {
    /// Does not validate:
    /// - that example pairings match schemas
    /// - that Example::value and Example::externalValue are mutually exclusive
    /// - $ref
    /// - links, runtime expressions
    /// - component keys are idents
    /// - error codes are unique
    Validate {
        path: PathBuf,
    },
    Diff {
        left: PathBuf,
        right: PathBuf,
    },
    Select {
        openrpc: PathBuf,
        select: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    match Args::parse() {
        Args::Openrpc(Openrpc::Validate { path }) => {
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
        Args::Openrpc(Openrpc::Diff { left, right }) => {
            let summary = openrpc_diff::diff(load_json(left)?, load_json(right)?)?;
            serde_json::to_writer_pretty(io::stdout(), &summary)?;
            Ok(())
        }
        Args::Openrpc(Openrpc::Select { openrpc, select }) => {
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
