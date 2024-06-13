use ascii::AsciiChar;
use clap::Parser;
use std::{collections::BTreeMap, fmt, io, str::FromStr};

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = Char(AsciiChar::Tab))]
    delimiter: Char,
}

#[derive(Clone)]
struct Char(AsciiChar);

impl FromStr for Char {
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

fn main() -> anyhow::Result<()> {
    let Args {
        delimiter: Char(delimiter),
    } = Args::parse();
    let mut records = csv::ReaderBuilder::new()
        .delimiter(delimiter.as_byte())
        .from_reader(io::stdin())
        .deserialize::<BTreeMap<String, String>>()
        .collect::<Result<Vec<_>, _>>()?;
    for record in &mut records {
        record.retain(|_k, v| !v.is_empty())
    }
    serde_json::to_writer_pretty(io::stdout(), &records)?;
    Ok(())
}
