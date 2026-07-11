use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use fluent_lint::{lint, render_ansi, render_plain};

const USAGE: &str = "Usage: fl-lint [OPTIONS] <FILE>...

Lint Fluent FTL resources and print compiler-style diagnostics.

Arguments:
  <FILE>...             FTL files to lint; use `-` to read standard input

Options:
      --color <WHEN>    Color output: auto, always, or never [default: auto]
      --no-color        Alias for `--color never`
  -h, --help            Print help
  -V, --version         Print version";

#[derive(Clone, Copy)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

struct Options {
    color: ColorChoice,
    paths: Vec<String>,
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
        let rendered = if ansi {
            render_ansi(&filename, &source, &diagnostics)
        } else {
            render_plain(&filename, &source, &diagnostics)
        };
        eprint!("{rendered}");
    }

    if io_failed {
        ExitCode::from(2)
    } else if found_diagnostics {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
