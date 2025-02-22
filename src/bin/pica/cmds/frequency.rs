use crate::config::Config;
use crate::skip_invalid_flag;
use crate::util::{App, CliArgs, CliError, CliResult};
use bstr::BString;
use clap::Arg;
use pica::{Path, ReaderBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FrequencyConfig {
    pub(crate) skip_invalid: Option<bool>,
}

pub(crate) fn cli() -> App {
    App::new("frequency")
        .about("Compute a frequency table of a subfield.")
        .arg(
            Arg::new("skip-invalid")
                .short('s')
                .long("skip-invalid")
                .help("skip invalid records"),
        )
        .arg(
            Arg::new("reverse")
                .short('r')
                .long("reverse")
                .help("Sort results in reverse order."),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("--limit")
                .value_name("n")
                .help("Limit the result to the <n> most common items."),
        )
        .arg(
            Arg::new("threshold")
                .short('t')
                .long("--threshold")
                .value_name("t")
                .help("Ignore rows with a frequency ≤ <t>."),
        )
        .arg(
            Arg::new("header")
                .short('H')
                .long("--header")
                .value_name("header")
                .help("Comma-separated list of column names."),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("--output")
                .value_name("file")
                .help("Write output to <file> instead of stdout."),
        )
        .arg(Arg::new("path").required(true))
        .arg(Arg::new("filename"))
}

fn writer(filename: Option<&str>) -> CliResult<Box<dyn Write>> {
    Ok(match filename {
        Some(filename) => Box::new(File::create(filename)?),
        None => Box::new(io::stdout()),
    })
}

pub(crate) fn run(args: &CliArgs, config: &Config) -> CliResult<()> {
    let skip_invalid =
        skip_invalid_flag!(args, config.frequency, config.global);

    let limit = match args.value_of("limit").unwrap_or("0").parse::<usize>() {
        Ok(limit) => limit,
        Err(_) => {
            return Err(CliError::Other(
                "Invalid limit value, expected unsigned integer.".to_string(),
            ));
        }
    };

    let threshold =
        match args.value_of("threshold").unwrap_or("0").parse::<u64>() {
            Ok(threshold) => threshold,
            Err(_) => {
                return Err(CliError::Other(
                    "Invalid threshold value, expected unsigned integer."
                        .to_string(),
                ));
            }
        };

    let mut writer =
        csv::WriterBuilder::new().from_writer(writer(args.value_of("output"))?);

    let mut reader = ReaderBuilder::new()
        .skip_invalid(skip_invalid)
        .from_path_or_stdin(args.value_of("filename"))?;

    let mut ftable: HashMap<BString, u64> = HashMap::new();
    let path = Path::from_str(args.value_of("path").unwrap())?;

    for result in reader.records() {
        let record = result?;

        for value in record.path(&path) {
            *ftable.entry(value.to_owned()).or_insert(0) += 1;
        }
    }

    if let Some(header) = args.value_of("header") {
        writer.write_record(header.split(',').map(|s| s.trim()))?;
    }

    let mut ftable_sorted: Vec<(&BString, &u64)> = ftable.iter().collect();
    if args.is_present("reverse") {
        ftable_sorted.sort_by(|a, b| a.1.cmp(b.1));
    } else {
        ftable_sorted.sort_by(|a, b| b.1.cmp(a.1));
    }

    for (i, (value, frequency)) in ftable_sorted.iter().enumerate() {
        if limit > 0 && i >= limit {
            break;
        }

        if **frequency <= threshold {
            break;
        }

        writer.write_record(&[value, &BString::from(frequency.to_string())])?;
    }

    writer.flush()?;
    Ok(())
}
