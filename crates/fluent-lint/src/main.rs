use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use fluent_lint::{Diagnostic, Severity, lint, render_ansi, render_plain};
use serde::Serialize;

const USAGE: &str = "Usage: fl-lint [OPTIONS] <FILE>...

Lint Fluent FTL resources and print compiler-style diagnostics.

Arguments:
  <FILE>...             FTL files to lint; use `-` to read standard input

Options:
      --color <WHEN>    Color output: auto, always, or never [default: auto]
      --no-color        Alias for `--color never`
      --format <FORMAT> Output format: human or json [default: human]
  -h, --help            Print help
  -V, --version         Print version";

#[derive(Clone, Copy)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum OutputFormat {
    Human,
    Json,
}

struct Options {
    color: ColorChoice,
    format: OutputFormat,
    paths: Vec<String>,
}

#[derive(Serialize)]
struct JsonDocument {
    schema_version: u32,
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    path: String,
    severity: &'static str,
    code: String,
    message: String,
    labels: Vec<JsonLabel>,
    notes: Vec<String>,
    help: Vec<String>,
}

#[derive(Serialize)]
struct JsonLabel {
    primary: bool,
    message: String,
    span: JsonSpan,
}

#[derive(Serialize)]
struct JsonSpan {
    start: JsonPosition,
    end: JsonPosition,
}

#[derive(Serialize)]
struct JsonPosition {
    byte: usize,
    line: usize,
    column: usize,
}

enum Command {
    Lint(Options),
    Help,
    Version,
}

fn main() -> ExitCode {
    let command = match parse_args(env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match command {
        Command::Help => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("fl-lint {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Lint(options) => run(options),
    }
}

fn parse_args(arguments: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut arguments = arguments.peekable();
    let mut options = Options {
        color: ColorChoice::Auto,
        format: OutputFormat::Human,
        paths: Vec::new(),
    };
    let mut positional_only = false;

    while let Some(argument) = arguments.next() {
        if positional_only {
            options.paths.push(argument);
            continue;
        }
        match argument.as_str() {
            "--" => positional_only = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--no-color" => options.color = ColorChoice::Never,
            "--format" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--format` requires human or json".to_owned())?;
                options.format = parse_format(&value)?;
            }
            "--color" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--color` requires auto, always, or never".to_owned())?;
                options.color = parse_color(&value)?;
            }
            "-" => options.paths.push(argument),
            _ if argument.starts_with("--color=") => {
                options.color = parse_color(&argument[8..])?;
            }
            _ if argument.starts_with("--format=") => {
                options.format = parse_format(&argument[9..])?;
            }
            _ if argument.starts_with('-') => {
                return Err(format!("unknown option `{argument}`"));
            }
            _ => options.paths.push(argument),
        }
    }

    if options.paths.is_empty() {
        return Err("at least one FTL file is required".into());
    }
    Ok(Command::Lint(options))
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "human" => Ok(OutputFormat::Human),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!(
            "invalid output format `{value}`; expected human or json"
        )),
    }
}

fn parse_color(value: &str) -> Result<ColorChoice, String> {
    match value {
        "auto" => Ok(ColorChoice::Auto),
        "always" => Ok(ColorChoice::Always),
        "never" => Ok(ColorChoice::Never),
        _ => Err(format!(
            "invalid color mode `{value}`; expected auto, always, or never"
        )),
    }
}

fn run(options: Options) -> ExitCode {
    let ansi = match options.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => io::stderr().is_terminal() && env::var_os("NO_COLOR").is_none(),
    };
    let mut stdin_used = false;
    let mut found_diagnostics = false;
    let mut io_failed = false;
    let mut json_diagnostics = Vec::new();

    for path in options.paths {
        let (filename, source) = if path == "-" {
            if stdin_used {
                eprintln!("error: standard input may only be specified once");
                io_failed = true;
                continue;
            }
            stdin_used = true;
            let mut source = String::new();
            if let Err(error) = io::stdin().read_to_string(&mut source) {
                eprintln!("error: cannot read standard input: {error}");
                io_failed = true;
                continue;
            }
            ("<stdin>".to_owned(), source)
        } else {
            match fs::read_to_string(&path) {
                Ok(source) => (path, source),
                Err(error) => {
                    eprintln!("error: cannot read `{path}`: {error}");
                    io_failed = true;
                    continue;
                }
            }
        };

        let diagnostics = lint(&source);
        if diagnostics.is_empty() {
            continue;
        }
        found_diagnostics = true;
        match options.format {
            OutputFormat::Human => {
                let rendered = if ansi {
                    render_ansi(&filename, &source, &diagnostics)
                } else {
                    render_plain(&filename, &source, &diagnostics)
                };
                eprint!("{rendered}");
            }
            OutputFormat::Json => {
                json_diagnostics.extend(json_diagnostics_for(&filename, &source, &diagnostics));
            }
        }
    }

    if options.format == OutputFormat::Json {
        let document = JsonDocument {
            schema_version: 1,
            diagnostics: json_diagnostics,
        };
        println!(
            "{}",
            serde_json::to_string(&document)
                .expect("serializing lint diagnostics to JSON cannot fail")
        );
    }

    if io_failed {
        ExitCode::from(2)
    } else if found_diagnostics {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn json_diagnostics_for(
    path: &str,
    source: &str,
    diagnostics: &[Diagnostic],
) -> Vec<JsonDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| JsonDiagnostic {
            path: path.to_owned(),
            severity: match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| JsonLabel {
                    primary: label.primary,
                    message: label.message.clone(),
                    span: JsonSpan {
                        start: source_position(source, label.span.start),
                        end: source_position(source, label.span.end),
                    },
                })
                .collect(),
            notes: diagnostic.notes.clone(),
            help: diagnostic.help.clone(),
        })
        .collect()
}

fn source_position(source: &str, byte: usize) -> JsonPosition {
    let mut line = 0;
    let mut column = 0;
    let mut characters = source[..byte].chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                line += 1;
                column = 0;
            }
            '\n' => {
                line += 1;
                column = 0;
            }
            _ => column += 1,
        }
    }

    JsonPosition { byte, line, column }
}
