use crate::config::Config;
use crate::skip_invalid_flag;
use crate::util::{App, CliArgs, CliError, CliResult};
use clap::Arg;
use pica::{Outcome, ReaderBuilder, Selectors};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Write};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct SelectConfig {
    pub(crate) skip_invalid: Option<bool>,
}

pub(crate) fn cli() -> App {
    App::new("select")
        .about("Select fields from a record.")
        .arg(
            Arg::new("skip-invalid")
                .short('s')
                .long("skip-invalid")
                .about("skip invalid records"),
        )
        .arg(
            Arg::new("no-empty-columns")
                .long("no-empty-columns")
                .about("disallow empty columns"),
        )
        .arg(
            Arg::new("ignore-case")
                .short('i')
                .long("--ignore-case")
                .about("When this flag is provided, comparision operations will be search case insensitive."),
        )
        .arg(
            Arg::new("tsv")
                .short('t')
                .long("tsv")
                .about("use tabs as field delimiter"),
        )
        .arg(
            Arg::new("header")
                .short('H')
                .long("--header")
                .value_name("header")
                .about("Comma-separated list of column names."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("--output")
                .value_name("file")
                .about("Write output to <file> instead of stdout."),
        )
        .arg(Arg::new("selectors").required(true))
        .arg(Arg::new("filename"))
}

fn writer(filename: Option<&str>) -> CliResult<Box<dyn Write>> {
    Ok(match filename {
        Some(filename) => Box::new(File::create(filename)?),
        None => Box::new(io::stdout()),
    })
}

pub(crate) fn run(args: &CliArgs, config: &Config) -> CliResult<()> {
    let skip_invalid = skip_invalid_flag!(args, config.select, config.global);
    let no_empty_columns = args.is_present("no-empty-columns");
    let ignore_case = args.is_present("ignore-case");

    let mut reader = ReaderBuilder::new()
        .skip_invalid(skip_invalid)
        .from_path_or_stdin(args.value_of("filename"))?;

    let mut writer = csv::WriterBuilder::new()
        .delimiter(if args.is_present("tsv") { b'\t' } else { b',' })
        .from_writer(writer(args.value_of("output"))?);

    let selectors_str = args.value_of("selectors").unwrap();
    let selectors = match Selectors::decode(selectors_str) {
        Ok(val) => val,
        _ => {
            return Err(CliError::Other(format!(
                "invalid select list: {}",
                selectors_str
            )))
        }
    };

    if let Some(header) = args.value_of("header") {
        writer.write_record(header.split(',').map(|s| s.trim()))?;
    }

    for result in reader.records() {
        let record = result?;
        let outcome = selectors
            .iter()
            .map(|selector| record.select(selector, ignore_case))
            .fold(Outcome::default(), |acc, x| acc * x);

        for row in outcome.iter() {
            if no_empty_columns && row.iter().any(|column| column.is_empty()) {
                continue;
            }

            if !row.iter().all(|col| col.is_empty()) {
                writer.write_record(row)?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}
