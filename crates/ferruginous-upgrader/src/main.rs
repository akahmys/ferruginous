//! Ferruginous Upgrader: Specialized CLI for PDF 2.0 Conversion.
//!
//! (ISO 32000-2:2020 Compliance Engine)

use clap::Parser;
use ferruginous_sdk::PdfDocument;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Parser, Debug)]
#[command(author, version, about = "Converts PDF documents to version 2.0", long_about = None)]
struct Args {
    /// Input PDF file path
    input: PathBuf,

    /// Output PDF file path
    output: PathBuf,

    /// Opt-in for PDF Linearization (Fast Web View / Web Optimization)
    #[arg(long)]
    linearize: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Ferruginous Upgrader v{} - Initializing...", env!("CARGO_PKG_VERSION"));
    println!("Source: {:?}", args.input);
    println!("Target: {:?}", args.output);

    // 1. Load the document
    let data = std::fs::read(&args.input)
        .with_context(|| format!("Failed to read input file: {:?}", args.input))?;
    let doc = PdfDocument::open(data.into())
        .map_err(|e| anyhow::anyhow!("PDF Load Error: {:?}", e))?;

    // 2. Perform the upgrade and save
    if args.linearize {
        println!("Upgrading to Linearized PDF 2.0 structure...");
        doc.save_linearized(&args.output, "2.0")
            .map_err(|e| anyhow::anyhow!("Linearization/Save Error: {:?}", e))?;
    } else {
        println!("Upgrading to Standard PDF 2.0 structure...");
        doc.save_as_version(&args.output, "2.0")
            .map_err(|e| anyhow::anyhow!("Upgrade/Save Error: {:?}", e))?;
    }

    if args.linearize {
        println!("SUCCESS: Document successfully upgraded and linearized (Fast Web View).");
    } else {
        println!("SUCCESS: Document successfully upgraded to PDF 2.0.");
    }
    println!("Output saved to: {:?}", args.output);

    Ok(())
}
