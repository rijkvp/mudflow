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

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Source files
    #[clap(short)]
    sources: Option<Vec<PathBuf>>,
    /// Output directory (required on template glob)
    #[clap(short)]
    out_dir: Option<PathBuf>,
    /// File format of the input (required on stdin source)
    #[clap(short, value_enum)]
    format: Option<FileFormat>,
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
    #[error("Unsupported file extension: {0}")]
    UnsupportedExt(String),
    #[error("{0}")]
    Msg(String),
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
    let sources: Vec<(FileFormat, String)> = {
        if let Some(sources) = args.sources {
            let mut inputs = Vec::new();
            for source_path in &sources {
                let format = if let Some(f) = args.format {
                    f
                } else {
                    // TODO: Handle errors
                    let ext = source_path.extension().unwrap();
                    FileFormat::from_ext(ext.to_str().unwrap())?
                };
                let source_str = std::fs::read_to_string(&source_path).map_err(|e| {
                    Error::IO(format!(
                        "Failed to read source file '{}': {}",
                        source_path.display(),
                        e
                    ))
                })?;
                inputs.push((format, source_str));
            }
            inputs
        } else {
            let format = args
                .format
                .ok_or(Error::Msg("Format required when using stdin!".to_string()))?;
            let mut stdin = io::stdin();
            let mut input_str = String::new();
            stdin
                .read_to_string(&mut input_str)
                .map_err(|e| Error::IO(format!("Failed to read stdin: {}", e)))?;
            vec![(format, input_str)]
        }
    };
    let context = deserialize(&sources)?;

    if let Some(out_dir) = args.out_dir {
        let mut tera = Tera::new(&args.templates)?;
        tera.autoescape_on(vec![]);
        std::fs::create_dir_all(&out_dir).map_err(|e| {
            Error::IO(format!(
                "Failed to create directories '{}': {}",
                out_dir.display(),
                e
            ))
        })?;
        for template in tera.get_template_names() {
            let path = out_dir.join(template);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(&parent).map_err(|e| {
                    Error::IO(format!(
                        "Failed to create directories '{}': {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
            let mut file = File::create(&path).map_err(|e| {
                Error::IO(format!("Failed to create file '{}': {}", path.display(), e))
            })?;
            tera.render_to(template, &context, &mut file)?;
            eprintln!(
                "{} {:<30}  {}  {:<30}",
                "Rendered".green().bold(),
                template.dimmed(),
                "->".white().bold(),
                path.display().dimmed()
            );
        }
    } else {
        let template_input = std::fs::read_to_string(&args.templates).map_err(|e| {
            Error::IO(format!(
                "Failed to read source file '{}': {}",
                args.templates, e
            ))
        })?;
        let mut tera = Tera::default();
        tera.autoescape_on(vec![]);
        tera.add_raw_template(&args.templates, &template_input)?;
        let mut stdout = io::stdout().lock();
        tera.render_to(&args.templates, &context, &mut stdout)?;
    }
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, ValueEnum)]
enum FileFormat {
    Json,
    Yaml,
    Toml,
}

impl FileFormat {
    fn from_ext(s: &str) -> Result<Self, Error> {
        return match s.trim().to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            _ => Err(Error::UnsupportedExt(s.to_string())),
        };
    }
}

fn deserialize(input: &Vec<(FileFormat, String)>) -> Result<Context, Error> {
    let mut context = Context::new();
    for (format, str) in input {
        let value: serde_json::Value = match format {
            FileFormat::Json => serde_json::from_str::<serde_json::Value>(str).map_err(|e| {
                Error::Deserialization(format!("Failed JSON deserialization: {}", e))
            })?,
            FileFormat::Yaml => serde_yaml::from_str::<serde_json::Value>(str).map_err(|e| {
                Error::Deserialization(format!("Failed YAML deserialization: {}", e))
            })?,
            FileFormat::Toml => toml::from_str::<serde_json::Value>(str).map_err(|e| {
                Error::Deserialization(format!("Failed TOML deserialization: {}", e))
            })?,
        };
        context.extend(Context::from_value(value)?);
    }
    Ok(context)
}
