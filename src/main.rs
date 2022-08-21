use clap::{Parser, ValueEnum};
use owo_colors::OwoColorize;
use std::{
    error::Error as StdError,
    fmt::Debug,
    fs,
    io::{self, Read},
    path::PathBuf,
};
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Debug, Clone, ValueEnum)]
enum FileFormat {
    JSON,
    YAML,
    TOML,
}

fn deserialize(input: &str, format: FileFormat) -> Result<Context, Error> {
    match format {
        FileFormat::JSON => Ok(Context::from_serialize(
            serde_json::from_str::<serde_json::Value>(&input).map_err(|e| {
                Error::Deserialization(format!("Failed JSON deserialization: {}", e))
            })?,
        )?),
        FileFormat::YAML => Ok(Context::from_serialize(
            serde_yaml::from_str::<serde_json::Value>(&input).map_err(|e| {
                Error::Deserialization(format!("Failed YAML deserialization: {}", e))
            })?,
        )?),
        FileFormat::TOML => Ok(Context::from_serialize(
            toml::from_str::<serde_json::Value>(&input).map_err(|e| {
                Error::Deserialization(format!("Failed TOML deserialization: {}", e))
            })?,
        )?),
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Read a file as input
    #[clap(short)]
    input_path: Option<PathBuf>,
    /// File format of the input
    #[clap(value_enum)]
    format: FileFormat,
    /// Path to the template file
    template: String,
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e.white())
        }
    }
}

#[derive(Error, Debug)]
enum Error {
    #[error("IO: {0}")]
    IO(String),
    #[error("{0}")]
    Deserialization(String),
    #[error("Template render: {0}")]
    Template(String),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl std::convert::From<tera::Error> for Error {
    fn from(err: tera::Error) -> Self {
        if let Some(source) = err.source() {
            Error::Template(format!("{}\n{}", err, source))
        } else {
            Error::Template(err.to_string())
        }
    }
}

fn run(args: Args) -> Result<String, Error> {
    let input = {
        if let Some(input_path) = args.input_path {
            fs::read_to_string(&input_path)?
        } else {
            let mut stdin = io::stdin();
            let mut input_buf = String::new();
            stdin.read_to_string(&mut input_buf)?;
            input_buf
        }
    };
    let template_input = fs::read_to_string(&args.template)?;
    let context = deserialize(&input, args.format)?;
    let result = Tera::one_off(&template_input, &context, false)?;
    Ok(result)
}
