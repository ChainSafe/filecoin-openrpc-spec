mod diff;
mod openrpc_types;
use anyhow::{bail, Context as _};
use clap::Parser;
use itertools::Itertools as _;
use openrpc_types::{Components, OpenRPC};
use std::{fs::File, path::PathBuf};

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
    Validate { path: PathBuf },
}

fn main() -> anyhow::Result<()> {
    match Args::parse() {
        Args::Openrpc(Openrpc::Validate { path }) => {
            let document = serde_path_to_error::deserialize::<_, OpenRPC>(
                &mut serde_json::Deserializer::from_reader(File::open(path)?),
            )?;
            if let Ok(dups) = nunny::Vec::new(
                document
                    .methods
                    .iter()
                    .map(|it| it.name.as_str())
                    .duplicates()
                    .collect(),
            ) {
                bail!(
                    "the following method names are duplicated: {}",
                    dups.join(", ")
                )
            };

            for method in &document.methods {
                if let Ok(dups) = nunny::Vec::new(
                    method
                        .params
                        .iter()
                        .map(|it| it.name.as_str())
                        .duplicates()
                        .collect(),
                ) {
                    bail!(
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
                        bail!("the following required parameters on method {} follow the optional parameter {}: {}", method.name, name, after.join(", "))
                    }
                }
            }

            Ok(())
        }
    }
}
