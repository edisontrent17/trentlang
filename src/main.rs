use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use trentlang::{compile_file, CompileOptions};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }

    let mut emit_ir_only = false;
    let mut output_path = None;
    let mut input_path = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--emit-ir-only" => {
                emit_ir_only = true;
                index += 1;
            }
            "--keep-ir" => {
                index += 1;
            }
            "-o" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("expected output path after -o".to_string());
                };
                output_path = Some(PathBuf::from(value));
                index += 1;
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => {
                if input_path.is_some() {
                    return Err("expected exactly one input file".to_string());
                }
                input_path = Some(PathBuf::from(value));
                index += 1;
            }
        }
    }

    let input_path = input_path.ok_or_else(|| "expected input file".to_string())?;
    let output = compile_file(
        &input_path,
        &CompileOptions {
            emit_ir_only,
            output_path,
        },
    )
    .map_err(|err| err.to_string())?;

    println!("wrote {}", output.ir_path.display());
    if let Some(executable_path) = output.executable_path {
        println!("wrote {}", executable_path.display());
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: trentlang [--emit-ir-only] [--keep-ir] [-o OUTPUT] INPUT.tl");
}
