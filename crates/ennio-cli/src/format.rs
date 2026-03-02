use std::fmt;
use std::str::FromStr;

use serde::Serialize;

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown format: {other}")),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Json => write!(f, "json"),
        }
    }
}

pub fn print_json<T: Serialize>(data: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{json}");
    Ok(())
}

pub fn print_table_header(columns: &[(&str, usize)]) {
    let header: String = columns
        .iter()
        .map(|(name, width)| format!("{name:<width$}"))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{header}");
    let separator: String = columns
        .iter()
        .map(|(_, width)| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{separator}");
}

pub fn print_table_row(values: &[(&str, usize)]) {
    let row: String = values
        .iter()
        .map(|(val, width)| format!("{val:<width$}"))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{row}");
}
