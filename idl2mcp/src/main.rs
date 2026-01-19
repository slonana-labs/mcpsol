//! idl2mcp CLI - Convert Anchor IDL to MCP schema

use anyhow::{Context, Result};
use clap::Parser;
use idl2mcp::convert_idl_to_mcp_json;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "idl2mcp")]
#[command(about = "Convert Anchor IDL to MCP schema format")]
#[command(version)]
struct Args {
    /// Input IDL file (use - for stdin)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Pretty print the output JSON
    #[arg(short, long)]
    pretty: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input
    let idl_json = match &args.input {
        Some(path) if path.to_string_lossy() == "-" => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .context("Failed to read from stdin")?;
            buf
        }
        Some(path) => {
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read {}", path.display()))?
        }
        None => {
            // Try to find IDL in common locations
            let candidates = [
                "target/idl/*.json",
                "idl.json",
                "target/types/*.ts", // TypeScript IDL
            ];

            let mut found = None;
            for pattern in candidates {
                if let Ok(paths) = glob::glob(pattern) {
                    for path in paths.flatten() {
                        if path.extension().map(|e| e == "json").unwrap_or(false) {
                            found = Some(path);
                            break;
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }

            match found {
                Some(path) => {
                    eprintln!("Using IDL: {}", path.display());
                    fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read {}", path.display()))?
                }
                None => {
                    eprintln!("No IDL file specified. Usage: idl2mcp -i <idl.json>");
                    eprintln!("Or pipe IDL via stdin: cat idl.json | idl2mcp -i -");
                    std::process::exit(1);
                }
            }
        }
    };

    // Convert
    let mcp_json = convert_idl_to_mcp_json(&idl_json)?;

    // Pretty print if requested
    let output = if args.pretty {
        let parsed: serde_json::Value = serde_json::from_str(&mcp_json)?;
        serde_json::to_string_pretty(&parsed)?
    } else {
        mcp_json
    };

    // Write output
    match &args.output {
        Some(path) => {
            fs::write(path, &output)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            eprintln!("Wrote MCP schema to {}", path.display());
        }
        None => {
            io::stdout().write_all(output.as_bytes())?;
            println!();
        }
    }

    Ok(())
}
