use clap::{Parser, ValueEnum};
use owo_colors::OwoColorize;
use std::{
    error::Error as StdError,
    fmt::Debug,
    fs::File,
    io::{self, Read},
    path::PathBuf,
};
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Debug, Clone, ValueEnum)]
enum FileFormat {
    Json,
    Yaml,
    Toml,
}

fn deserialize(input: &str, format: FileFormat) -> Result<Context, Error> {
    match format {
        FileFormat::Json => Ok(Context::from_serialize(
            serde_json::from_str::<serde_json::Value>(input).map_err(|e| {
                Error::Deserialization(format!("Failed JSON deserialization: {}", e))
            })?,
        )?),
        FileFormat::Yaml => Ok(Context::from_serialize(
            serde_yaml::from_str::<serde_json::Value>(input).map_err(|e| {
                Error::Deserialization(format!("Failed YAML deserialization: {}", e))
            })?,
        )?),
        FileFormat::Toml => Ok(Context::from_serialize(
            toml::from_str::<serde_json::Value>(input).map_err(|e| {
                Error::Deserialization(format!("Failed TOML deserialization: {}", e))
            })?,
        )?),
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Input source file
    #[clap(short)]
    input_source: Option<PathBuf>,
    /// Output dir (required on template glob)
    #[clap(short)]
    output_dir: Option<PathBuf>,
    /// File format of the input
    #[clap(value_enum)]
    format: FileFormat,
    /// Template file or glob
    templates: String,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("{} {}", "error:".red().bold(), e.white());
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

impl std::convert::From<tera::Error> for Error {
    fn from(err: tera::Error) -> Self {
        if let Some(source) = err.source() {
            Error::Template(format!("{}\n{}", err, source))
        } else {
            Error::Template(err.to_string())
        }
    }
}

fn run(args: Args) -> Result<(), Error> {
    let input = {
        if let Some(input_source) = args.input_source {
            std::fs::read_to_string(&input_source).map_err(|e| Error::IO(format!("Failed to read input source '{}': {}", input_source.display(), e)))?
        } else {
            let mut stdin = io::stdin();
            let mut input_buf = String::new();
            stdin.read_to_string(&mut input_buf).map_err(|e| Error::IO(format!("Failed to read stdin: {}", e)))?;
            input_buf
        }
    };
    let context = deserialize(&input, args.format)?;

    if let Some(out_dir) = args.output_dir {
        let mut tera = Tera::new(&args.templates)?;
        tera.autoescape_on(vec![]);
        std::fs::create_dir_all(&out_dir).map_err(|e| Error::IO(format!("Failed to create directories '{}': {}", out_dir.display(), e)))?;
        for template in tera.get_template_names() {
            let path = out_dir.join(template);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(&parent).map_err(|e| Error::IO(format!("Failed to create directories '{}': {}", parent.display(), e)))?;
            }
            let mut file = File::create(&path).map_err(|e| Error::IO(format!("Failed to create file '{}': {}", path.display(), e)))?;
            tera.render_to(template, &context, &mut file)?;
            println!("{} {:<30}  {}  {:<30}", "Rendered".green().bold(), template.dimmed(), "->".white().bold(), path.display().dimmed());
        }
    } else {
        let template_input = std::fs::read_to_string(&args.templates).map_err(|e| Error::IO(format!("Failed to read file '{}': {}", args.templates, e)))?;
        let mut tera = Tera::default();
        tera.autoescape_on(vec![]);
        tera.add_raw_template(&args.templates, &template_input)?;
        let mut stdout = io::stdout().lock();
        tera.render_to(&args.templates, &context, &mut stdout)?;
    }
    Ok(())
}
