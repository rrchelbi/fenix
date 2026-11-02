mod ast;
mod check;
mod codegen;
mod lexer;
mod parser;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use lexer::Error;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (command, path) = match args.as_slice() {
        [command, path] => (command.as_str(), path.as_str()),
        _ => {
            eprintln!("usage: fenix <run|build|c> <file.fx>");
            process::exit(1);
        }
    };

    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        process::exit(1);
    });
    let c_code = compile(&source).unwrap_or_else(|error| {
        eprintln!("{}", render(path, &source, &error));
        process::exit(1);
    });

    match command {
        "c" => print!("{c_code}"),
        "build" => {
            let output = Path::new(path).with_extension(env::consts::EXE_EXTENSION);
            build(&c_code, &output);
            println!("built {}", output.display());
        }
        "run" => {
            let output = env::temp_dir()
                .join(format!("fenix-{}", process::id()))
                .with_extension(env::consts::EXE_EXTENSION);
            build(&c_code, &output);
            let status = Command::new(&output)
                .status()
                .expect("could not run the program");
            let _ = fs::remove_file(&output);
            process::exit(status.code().unwrap_or(1));
        }
        _ => {
            eprintln!("unknown command `{command}`");
            process::exit(1);
        }
    }
}

fn compile(source: &str) -> Result<String, Error> {
    let tokens = lexer::lex(source)?;
    let program = parser::Parser::new(tokens).program()?;
    check::check(&program)?;
    Ok(codegen::generate(&program))
}

/// Writes the C to a temporary file and asks clang for a binary.
fn build(c_code: &str, output: &Path) {
    let c_file = env::temp_dir().join(format!("fenix-{}.c", process::id()));
    fs::write(&c_file, c_code).expect("could not write the C file");
    let result = Command::new(clang())
        .args(["-O2", "-fwrapv", "-o"])
        .arg(output)
        .arg(&c_file)
        .output()
        .expect("could not start clang; is it installed?");
    let _ = fs::remove_file(&c_file);
    if !result.status.success() {
        eprintln!("clang failed:\n{}", String::from_utf8_lossy(&result.stderr));
        process::exit(1);
    }
}

/// clang on PATH, or where the LLVM installer puts it on Windows.
fn clang() -> PathBuf {
    if Command::new("clang").arg("--version").output().is_ok() {
        return PathBuf::from("clang");
    }
    PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe")
}

/// Shows an error with its line, column, and a caret under the problem.
fn render(path: &str, source: &str, error: &Error) -> String {
    let start = error.span.start.min(source.len());
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = source[start..]
        .find('\n')
        .map_or(source.len(), |i| start + i);
    let line_number = source[..start].matches('\n').count() + 1;
    let column = source[line_start..start].chars().count() + 1;
    let line = source[line_start..line_end].trim_end_matches('\r');
    let width = (error.span.end.min(line_end).saturating_sub(start)).max(1);
    format!(
        "{path}:{line_number}:{column}: error: {}\n  {line}\n  {}{}",
        error.message,
        " ".repeat(column - 1),
        "^".repeat(width)
    )
}
