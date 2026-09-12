use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

mod address;

use address::{Address, ParseError, ValidationMode};

fn main() -> ExitCode {
    let mut json_mode = false;
    let mut multi = false;
    let mut zip5 = false;
    let mut mode = ValidationMode::Strict;
    let mut path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" => json_mode = true,
            "--multi" => multi = true,
            "--zip5" => zip5 = true,
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

    if multi {
        let mut results = address::parse_all_with_mode(&input, mode);
        if results.is_empty() {
            eprintln!("no addresses found in input");
            return ExitCode::FAILURE;
        }
        if zip5 {
            for result in &mut results {
                if let Ok(addr) = result {
                    addr.truncate_zip_to_five();
                }
            }
        }
        let all_ok = if json_mode {
            print_multi_json(&results)
        } else {
            print_multi_text(&results)
        };
        return if all_ok { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }

    match address::parse_with_mode(&input, mode) {
        Ok(mut addr) => {
            if zip5 {
                addr.truncate_zip_to_five();
            }
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

/// Prints each address on its own paragraph, in input order, and sends
/// parse errors to stderr tagged with their 1-based position so a batch of
/// mostly-good input doesn't get lost behind one bad entry.
fn print_multi_text(results: &[Result<Address, ParseError>]) -> bool {
    let mut all_ok = true;
    let mut printed_any = false;
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(addr) => {
                if printed_any {
                    println!();
                }
                println!("{}", addr.to_pretty());
                printed_any = true;
            }
            Err(e) => {
                all_ok = false;
                eprintln!("address {}: {e}", i + 1);
            }
        }
    }
    all_ok
}

/// Renders every result, success or error, as one JSON array in input order
/// so a caller can line results back up with whatever it fed in.
fn print_multi_json(results: &[Result<Address, ParseError>]) -> bool {
    let mut all_ok = true;
    let mut items = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(addr) => items.push(addr.to_json()),
            Err(e) => {
                all_ok = false;
                items.push(e.to_json());
            }
        }
    }
    println!("[{}]", items.join(","));
    all_ok
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
    eprintln!("usage: address-tool [--json] [--multi] [--zip5] [--strict|--lenient] [FILE]");
    eprintln!();
    eprintln!("Reads a US postal address (street line(s), then \"City, ST ZIP\")");
    eprintln!("from FILE, or from stdin if FILE is omitted, validates it, and");
    eprintln!("prints it back out in canonical form.");
    eprintln!();
    eprintln!("--strict (default) requires the exact \"City, ST ZIP\" shape.");
    eprintln!("--lenient also accepts a missing comma before the state and a");
    eprintln!("zip that's short a leading zero.");
    eprintln!();
    eprintln!("--zip5 drops the +4 extension from the zip before printing, so");
    eprintln!("\"95014-2083\" comes out as \"95014\". Has no effect on a zip that");
    eprintln!("was already 5 digits.");
    eprintln!();
    eprintln!("--multi treats the input as one or more addresses separated by");
    eprintln!("blank lines. Each is parsed independently; a bad one is reported");
    eprintln!("(to stderr in text mode, inline in the JSON array) without");
    eprintln!("stopping the rest, and the exit code reflects whether all of");
    eprintln!("them parsed cleanly.");
}
