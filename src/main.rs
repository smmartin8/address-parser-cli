use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

mod address;

use address::ValidationMode;

fn main() -> ExitCode {
    let mut json_mode = false;
    let mut mode = ValidationMode::Strict;
    let mut path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" => json_mode = true,
            "--strict" => mode = ValidationMode::Strict,
            "--lenient" => mode = ValidationMode::Lenient,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {other}");
                    print_usage();
                    return ExitCode::FAILURE;
                }
                path = Some(other.to_string());
            }
        }
    }

    let input = match read_input(path.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading input: {e}");
            return ExitCode::FAILURE;
        }
    };

    match address::parse_with_mode(&input, mode) {
        Ok(addr) => {
            if json_mode {
                println!("{}", addr.to_json());
            } else {
                println!("{}", addr.to_pretty());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if json_mode {
                println!("{}", e.to_json());
            } else {
                eprintln!("{e}");
            }
            ExitCode::FAILURE
        }
    }
}

fn read_input(path: Option<&str>) -> io::Result<String> {
    match path {
        Some(p) => fs::read_to_string(p),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn print_usage() {
    eprintln!("usage: address-tool [--json] [--strict|--lenient] [FILE]");
    eprintln!();
    eprintln!("Reads a US postal address (street line(s), then \"City, ST ZIP\")");
    eprintln!("from FILE, or from stdin if FILE is omitted, validates it, and");
    eprintln!("prints it back out in canonical form.");
    eprintln!();
    eprintln!("--strict (default) requires the exact \"City, ST ZIP\" shape.");
    eprintln!("--lenient also accepts a missing comma before the state and a");
    eprintln!("zip that's short a leading zero.");
}
