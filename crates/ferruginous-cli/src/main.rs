//! Ferruginous CLI: Command-line interface for high-performance PDF processing.
//!
//! Provides tools for document analysis, rendering, auditing, and content extraction.

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use ferruginous_sdk::PdfDocument;
use anyhow::{Context, Result};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "ferruginous")]
#[command(about = "High-performance ISO 32000-2 compliant PDF toolkit", after_help = "ISO 32000-2:2020 Compliance Enforcement Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show metadata and structure info of a PDF file
    Info {
        /// The PDF file to analyze
        file: PathBuf,
    },
    /// Render PDF pages to images
    Render {
        /// The PDF file to render
        file: PathBuf,
        /// The page index to render (0-based)
        #[arg(short, long, default_value_t = 0)]
        page: usize,
        /// Output path for the image
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Audit PDF for standards compliance (PDF/A, PDF/X, etc.)
    Audit {
        /// The PDF file to audit
        file: PathBuf,
    },
    /// Extract content from PDF
    Extract {
        #[command(subcommand)]
        target: ExtractTarget,
    },
}

#[derive(Subcommand)]
enum ExtractTarget {
    /// Extract text from the PDF
    Text {
        /// The PDF file to extract from
        file: PathBuf,
        /// Optional page index
        #[arg(short, long)]
        page: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => {
            println!("{} Analyzing {}", "info:".blue().bold(), file.display().to_string().underline());
            let data = std::fs::read(&file).context("Failed to read PDF file")?;
            let doc = PdfDocument::open(data.into()).context("Failed to open PDF document")?;
            
            println!("{:<20} {}", "Pages:".bold(), doc.page_count()?);
            
            if let Ok(compliance) = doc.get_compliance() {
                println!("\n{}", "--- Compliance & Standards ---".yellow());
                if let Some(p) = compliance.metadata.pdf_a_part {
                    println!("{:<20} PDF/A-{} (Level {})", "Standard:".bold(), p, compliance.metadata.pdf_a_conformance.unwrap_or_else(|| "unknown".into()));
                }
                if let Some(v) = compliance.metadata.pdf_x_version {
                    println!("{:<20} {}", "Standard:".bold(), v);
                }
                if let Some(p) = compliance.metadata.pdf_ua_part {
                    println!("{:<20} PDF/UA-{}", "Accessibility:".bold(), p);
                }
                println!("{:<20} {}", "Tagged PDF:".bold(), if compliance.has_struct_tree { "Yes".green() } else { "No".red() });
            }
        }
        Commands::Render { file, page, output } => {
            let out_path = output.unwrap_or_else(|| {
                let mut p = file.clone();
                p.set_extension(format!("page{page}.png"));
                p
            });

            println!("{} Rendering page {} of {}...", "render:".green().bold(), page, file.display());
            
            let data = std::fs::read(&file).context("Failed to read PDF file")?;
            let doc = PdfDocument::open(data.into()).context("Failed to open PDF document")?;
            
            doc.render_page_to_file(page, &out_path).await.context("Failed to render page")?;
            println!("{} Image saved to {}", "Success!".green().bold(), out_path.display().to_string().italic());
        }
        Commands::Audit { file } => {
            println!("{} Auditing {} for compliance...", "audit:".yellow().bold(), file.display());
            let data = std::fs::read(&file).context("Failed to read PDF file")?;
            let doc = PdfDocument::open(data.into()).context("Failed to open PDF document")?;
            let compliance = doc.get_compliance()?;
            
            println!("{compliance:#?}");
        }
        Commands::Extract { target } => {
            match target {
                ExtractTarget::Text { file, page } => {
                    println!("{} Extracting text from {}...", "extract:".cyan().bold(), file.display());
                    let data = std::fs::read(&file).context("Failed to read PDF file")?;
                    let doc = PdfDocument::open(data.into()).context("Failed to open PDF document")?;
                    
                    if let Some(p_idx) = page {
                        let text = doc.extract_text(p_idx).context("Failed to extract text from page")?;
                        println!("{text}");
                    } else {
                        let count = doc.page_count()?;
                        for i in 0..count {
                            println!("{}", format!("--- Page {i} ---").dimmed());
                            let text = doc.extract_text(i).context(format!("Failed to extract text from page {i}"))?;
                            println!("{text}");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
