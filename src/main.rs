use clap::{Parser, ValueEnum};
use owo_colors::OwoColorize;
use std::{
    fmt::{Debug, Display},
    fs,
    io::{self, Read},
};
use tera::{Context, Tera};

#[derive(Debug, Clone, ValueEnum)]
enum FileFormat {
    JSON,
    YAML,
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Read a file as input instead of stdin
    #[clap(short, long)]
    input: Option<String>,
    /// File format of the input
    #[clap(value_enum)]
    format: FileFormat,
    /// Path to the template file
    template: String,
}

fn main() {
    match run() {
        Ok(result) => println!("{}", result),
        Err(err) => {
            eprintln!(
                "{}{}{}",
                err.r#type.red().bold(),
                ": ".red().bold(),
                err.msg
            );
        }
    }
}

struct Error {
    r#type: ErrorType,
    msg: String,
}

impl Error {
    fn new(r#type: ErrorType, msg: String) -> Self {
        Self { r#type, msg }
    }
}

enum ErrorType {
    InputError,
    TemplateLoad,
    InputParse,
    RenderError,
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::InputError => f.write_str("input error"),
            ErrorType::TemplateLoad => f.write_str("template load error"),
            ErrorType::InputParse => f.write_str("input parse error"),
            ErrorType::RenderError => f.write_str("template render error"),
        }
    }
}

fn run() -> Result<String, Error> {
    let args = Args::parse();
    let input = {
        if let Some(input_file) = args.input {
            fs::read_to_string(&input_file).map_err(|err| {
                Error::new(
                    ErrorType::InputError,
                    format!("Failed to read file '{}': {}", input_file, err),
                )
            })
        } else {
            let mut stdin = io::stdin();
            let mut input_buf = String::new();
            stdin.read_to_string(&mut input_buf).map_err(|err| {
                Error::new(
                    ErrorType::InputError,
                    format!("Failed to read stdin: {}", err),
                )
            })?;
            Ok(input_buf)
        }
    }?;
    let template_input = fs::read_to_string(&args.template).map_err(|err| {
        Error::new(
            ErrorType::TemplateLoad,
            format!("Failed to load template '{}': {}", args.template, err),
        )
    })?;

    let context = {
        match args.format {
            FileFormat::JSON => {
                let json = serde_json::from_str::<serde_json::Value>(&input).map_err(|err| {
                    Error::new(
                        ErrorType::InputParse,
                        format!("Failed to load JSON: {}", err),
                    )
                })?;
                Context::from_serialize(json)
                    .map_err(|_| Error::new(ErrorType::InputParse, String::from("???")))?
            }
            FileFormat::YAML => {
                let yaml = serde_yaml::from_str::<serde_json::Value>(&input).map_err(|err| {
                    Error::new(
                        ErrorType::InputParse,
                        format!("Failed to load YAML: {}", err),
                    )
                })?;
                Context::from_serialize(yaml)
                    .map_err(|_| Error::new(ErrorType::InputParse, String::from("???")))?
            }
        }
    };
    // (TEMPFIX) TODO: Add proper error messages
    let result = Tera::one_off(&template_input, &context, false).unwrap();
    Ok(result)
}
 