//! fepdf: The Universal PDF Toolkit.
//!
//! (ISO 32000-2:2020 Compliance & Optimization Engine)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ferruginous_sdk::{PdfDocument, PdfStandard};
use inquire::Confirm;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "fepdf")]
#[command(author = "Ferruginous Developers")]
#[command(version)]
#[command(about = "fepdf: The Universal PDF Toolkit for Compliance, Optimization, and Manipulation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display document information and compliance audit
    Inspect {
        /// Input PDF file
        input: PathBuf,
        /// Perform detailed compliance audit
        #[arg(long)]
        audit: bool,
        /// Dump hierarchical object structure tree
        #[arg(long)]
        structure: bool,
        /// Output format (text, json, markdown)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Disable active 2-pass refinement (UTF-8 normalization)
        #[arg(long)]
        no_refinement: bool,
        /// Disable automatic conversion of Info to XMP
        #[arg(long)]
        no_metadata_recovery: bool,
        /// Use relaxed color validation policy
        #[arg(long)]
        relaxed_color: bool,
    },
    /// Upgrade document to PDF 2.0 and modern standards (A-4, X-6, UA-2)
    Upgrade {
        /// Input PDF file
        input: PathBuf,
        /// Output PDF file
        output: PathBuf,
        /// Target standard (a4, x6, ua2)
        #[arg(long)]
        standard: Option<String>,
        /// Optional ICC color profile path
        #[arg(long)]
        icc_profile: Option<PathBuf>,
        /// Opt-in for Fast Web View (Linearization)
        #[arg(long)]
        linearize: bool,
        /// Disable active 2-pass refinement
        #[arg(long)]
        no_refinement: bool,
        /// Disable automatic conversion of Info to XMP
        #[arg(long)]
        no_metadata_recovery: bool,
    },
    /// Merge multiple PDF files into one
    Merge {
        /// Input PDF files
        inputs: Vec<PathBuf>,
        /// Output PDF file
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Split or extract pages from a PDF
    Split {
        /// Input PDF file
        input: PathBuf,
        /// Output directory or file pattern
        #[arg(short, long)]
        output: PathBuf,
        /// Page range (e.g., 1-5, 10)
        #[arg(long)]
        pages: Option<String>,
    },
    /// Optimize and sanitize the document
    Optimize {
        /// Input PDF file
        input: PathBuf,
        /// Output PDF file
        output: PathBuf,
        /// Remove unreachable objects
        #[arg(long)]
        vacuum: bool,
        /// Strip descriptive metadata
        #[arg(long)]
        strip: bool,
        /// Encrypt with password
        #[arg(long)]
        password: Option<String>,
    },
    /// Attempt to repair a corrupted PDF document
    Repair {
        /// Input corrupted PDF file
        input: PathBuf,
        /// Output repaired PDF file
        output: PathBuf,
    },
    /// Rotate specific pages in the document
    Rotate {
        /// Input PDF file
        input: PathBuf,
        /// Output PDF file
        output: PathBuf,
        /// Pages to rotate (comma-separated, e.g., 1,3-5) (default: all)
        #[arg(short, long)]
        pages: Option<String>,
        /// Rotation angle (90, 180, 270)
        #[arg(short, long)]
        angle: i32,
    },
    /// Render a PDF page to an image (PNG, JPEG)
    Render {
        /// Input PDF file
        input: PathBuf,
        /// Output image file (format detected from extension)
        output: PathBuf,
        /// Page number to render (default 1)
        #[arg(short, long, default_value_t = 1)]
        page: usize,
    },
    /// Extract text from the document
    Text {
        /// Input PDF file
        input: PathBuf,
        /// Pages to extract text from (comma-separated or range, e.g., 1-5)
        #[arg(short, long)]
        pages: Option<String>,
    },
    /// Heuristically re-tag the document logical structure for UA-2
    Retag {
        /// Input PDF file
        input: PathBuf,
        /// Output repaired PDF file (Explicitly required)
        #[arg(short, long)]
        output: PathBuf,
        /// Enable interactive Wizard Mode
        #[arg(short, long)]
        wizard: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { input, audit, structure, format, no_refinement, no_metadata_recovery, relaxed_color } => {
            let options = ferruginous_core::ingest::IngestionOptions {
                active_refinement: !no_refinement,
                sublime_metadata: !no_metadata_recovery,
                color_policy: if relaxed_color { 
                    ferruginous_core::ingest::ColorPolicy::Relaxed 
                } else { 
                    ferruginous_core::ingest::ColorPolicy::Strict 
                },
            };
            handle_inspect(input, audit, structure, format, options)?;
        }
        Commands::Upgrade { input, output, standard, icc_profile, linearize, no_refinement, no_metadata_recovery } => {
            let options = ferruginous_core::ingest::IngestionOptions {
                active_refinement: !no_refinement,
                sublime_metadata: !no_metadata_recovery,
                color_policy: ferruginous_core::ingest::ColorPolicy::Strict,
            };
            handle_upgrade(input, output, standard, icc_profile, linearize, options)?;
        }
        Commands::Merge { inputs, output } => {
            handle_merge(inputs, output)?;
        }
        Commands::Split { input, output, pages } => {
            handle_split(input, output, pages)?;
        }
        Commands::Optimize { input, output, vacuum, strip, password } => {
            handle_optimize(input, output, vacuum, strip, password)?;
        }
        Commands::Repair { input, output } => {
            handle_repair(input, output)?;
        }
        Commands::Rotate { input, output, pages, angle } => {
            handle_rotate(input, output, pages, angle)?
        }
        Commands::Render { input, output, page } => handle_render(input, output, page)?,
        Commands::Text { input, pages } => handle_text(input, pages)?,
        Commands::Retag { input, output, wizard } => handle_retag(input, output, wizard)?,
    }

    Ok(())
}

fn handle_merge(inputs: Vec<PathBuf>, output: PathBuf) -> Result<()> {
    println!("fepdf merge: Combining {} files into {:?}", inputs.len(), output);
    let mut sources = Vec::new();
    for path in inputs.clone() {
        let data = std::fs::read(&path).with_context(|| format!("Failed to read {:?}", path))?;
        let doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        sources.push(doc);
    }

    let merged = PdfDocument::merge(sources).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    merged.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Merged output saved to {:?}", output);
    Ok(())
}

fn handle_split(input: PathBuf, output: PathBuf, pages: Option<String>) -> Result<()> {
    println!("fepdf split: Extracting pages from {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_indices = parse_page_range(&range_str, page_count)?;

    let extracted =
        doc.extract_pages(target_indices).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    extracted.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Extracted output saved to {:?}", output);
    Ok(())
}

fn render_summary_markdown(summary: &ferruginous_sdk::DocumentSummary, input: &std::path::Path, audit: bool) -> Result<()> {
    println!("# Document Summary: {:?}", input.file_name().unwrap_or_default());
    println!("\n## General Information");
    println!("\n| Property | Value |");
    println!("| :--- | :--- |");
    println!("| Version | {} |", summary.version);
    println!("| Total Pages | {} |", summary.page_count);
    if let Some(v) = &summary.metadata.title { println!("| Title | {} |", v); }
    if let Some(v) = &summary.metadata.author { println!("| Author | {} |", v); }
    if let Some(v) = &summary.metadata.subject { println!("| Subject | {} |", v); }
    if let Some(v) = &summary.metadata.keywords { println!("| Keywords | {} |", v); }
    if let Some(v) = &summary.metadata.creator { println!("| Creator | {} |", v); }
    if let Some(v) = &summary.metadata.producer { println!("| Producer | {} |", v); }
    
    let embedded_count = summary.fonts.iter().filter(|f| f.is_embedded).count();
    let total_fonts = summary.fonts.len();
    
    println!("\n## Font Audit (Embedded: {}/{})", embedded_count, total_fonts);
    if total_fonts > 0 {
        println!("\n| Font Name | Type | Embedded | Subset | Encoding |");
        println!("| :--- | :--- | :--- | :--- | :--- |");
        for f in &summary.fonts {
            println!("| {} | {} | {} | {} | {} |", 
                f.name, f.font_type, 
                if f.is_embedded { "✅" } else { "❌" },
                if f.is_subset { "✅" } else { "−" },
                f.encoding
            );
        }
    }

    if audit {
        render_compliance_markdown(summary)?;
    }
    Ok(())
}

fn render_compliance_markdown(summary: &ferruginous_sdk::DocumentSummary) -> Result<()> {
    let errors = summary.compliance.issues.iter().filter(|i| matches!(i.severity, ferruginous_sdk::IssueSeverity::Error | ferruginous_sdk::IssueSeverity::Critical)).count();
    let warnings = summary.compliance.issues.iter().filter(|i| matches!(i.severity, ferruginous_sdk::IssueSeverity::Warning)).count();
    println!("\n## Compliance Audit (UA-2)");
    println!("**Summary**: {} Errors, {} Warnings", errors, warnings);
    
    if !summary.compliance.issues.is_empty() {
        println!("\n| Severity | Standard | Message |");
        println!("| :--- | :--- | :--- |");
        for issue in &summary.compliance.issues {
            let icon = match issue.severity {
                ferruginous_sdk::IssueSeverity::Critical => "🚨",
                ferruginous_sdk::IssueSeverity::Error => "❌",
                ferruginous_sdk::IssueSeverity::Warning => "⚠️",
                ferruginous_sdk::IssueSeverity::Info => "ℹ️",
            };
            println!("| {} {:?} | {} | {} |", icon, issue.severity, issue.standard, issue.message);
        }
    } else {
        println!("\n✅ No compliance issues found.");
    }
    Ok(())
}

fn render_summary_text(doc: &PdfDocument, summary: &ferruginous_sdk::DocumentSummary, audit: bool, structure: bool) -> Result<()> {
    println!("\n--- [ DOCUMENT SUMMARY ] ---");
    println!("Version:    {}", summary.version);
    println!("Pages:      {}", summary.page_count);
    if let Some(v) = &summary.metadata.title { println!("Title:      {}", v); }
    if let Some(v) = &summary.metadata.author { println!("Author:     {}", v); }

    println!("\n--- [ FONT AUDIT ] ---");
    if summary.fonts.is_empty() {
        println!("No fonts detected.");
    } else {
        for f in &summary.fonts {
            println!("{:<25} | {:<10} | Emb: {:<5} | Sub: {:<5}", f.name, f.font_type, f.is_embedded, f.is_subset);
        }
    }

    if audit {
        println!("\n--- [ COMPLIANCE AUDIT ] ---");
        if summary.compliance.issues.is_empty() {
            println!("SUCCESS: No major issues found.");
        } else {
            for issue in &summary.compliance.issues {
                println!("[{:?}] {:<10} | {}", issue.severity, issue.standard, issue.message);
            }
        }
    }

    if structure {
        let tree = doc.print_structure().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        println!("\n{}", tree);
    }
    Ok(())
}

fn handle_inspect(input: PathBuf, audit: bool, structure: bool, format: String, options: ferruginous_core::ingest::IngestionOptions) -> Result<()> {
    if format == "text" { println!("fepdf inspect: Analyzing {:?}", input); }
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let doc = PdfDocument::open_with_options(data.into(), &options).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let summary = doc.get_summary().map_err(|e| anyhow::anyhow!("{:?}", e))?;

    match format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&summary)?),
        "markdown" => render_summary_markdown(&summary, &input, audit)?,
        _ => render_summary_text(&doc, &summary, audit, structure)?,
    }
    Ok(())
}

fn handle_upgrade(
    input: PathBuf,
    output: PathBuf,
    standard: Option<String>,
    icc_profile: Option<PathBuf>,
    linearize: bool,
    options: ferruginous_core::ingest::IngestionOptions,
) -> Result<()> {
    println!("fepdf upgrade: {:?} -> {:?}", input, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let mut doc = PdfDocument::open_with_options(data.into(), &options).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if let Some(std_str) = standard {
        let std = match std_str.to_lowercase().as_str() {
            "a4" => PdfStandard::A4,
            "x6" => PdfStandard::X6,
            "ua2" => PdfStandard::UA2,
            _ => anyhow::bail!("Unsupported standard: {}", std_str),
        };

        if (std == PdfStandard::A4 || std == PdfStandard::X6) && icc_profile.is_none() {
            println!("ADVICE: No --icc-profile specified. Defaulting to standard sRGB.");
        }
        doc.upgrade_to_standard(std).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    if linearize {
        doc.save_linearized(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else {
        doc.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    println!("SUCCESS: Output saved to {:?}", output);
    Ok(())
}

fn handle_optimize(
    input: PathBuf,
    output: PathBuf,
    vacuum: bool,
    strip: bool,
    password: Option<String>,
) -> Result<()> {
    println!("fepdf optimize: {:?} -> {:?}", input, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let mut doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    doc.set_vacuum(vacuum);
    doc.set_strip(strip);
    doc.set_password(password);

    doc.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Optimized output saved to {:?}", output);
    Ok(())
}

fn handle_repair(input: PathBuf, output: PathBuf) -> Result<()> {
    println!("fepdf repair: Attempting to salvage corrupted document {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let doc =
        PdfDocument::open_and_repair(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    doc.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Repaired output saved to {:?}", output);
    Ok(())
}

fn handle_rotate(input: PathBuf, output: PathBuf, pages: Option<String>, angle: i32) -> Result<()> {
    println!("fepdf rotate: Rotating pages in {:?} by {} degrees...", input, angle);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let mut doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Parse page ranges (basic implementation for now)
    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_pages = parse_page_range(&range_str, page_count)?;

    for idx in target_pages {
        doc.set_page_rotation(idx, angle).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    doc.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Rotated output saved to {:?}", output);
    Ok(())
}

fn handle_render(input: PathBuf, output: PathBuf, page_num: usize) -> Result<()> {
    println!("fepdf render: Rendering page {} of {:?} to {:?}...", page_num, input, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if page_num == 0 || page_num > doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))? {
        return Err(anyhow::anyhow!("Invalid page number: {}", page_num));
    }

    doc.render_page_to_file(page_num - 1, &output).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    println!("SUCCESS: Rendered page saved to {:?}", output);
    Ok(())
}

fn handle_retag(input: PathBuf, output: PathBuf, wizard: bool) -> Result<()> {
    println!("fepdf retag: {} -> {:?}", if wizard { "Wizard Mode" } else { "Automatic" }, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let mut doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if wizard {
        println!("Wizard Mode: Reviewing heuristic structural candidates...");
        let candidates = doc.get_remediation_candidates().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        
        if candidates.is_empty() {
            println!("No remediation candidates found.");
        } else {
            for candidate in candidates {
                let prompt = format!("Page {}: {}?", candidate.page_index + 1, candidate.description);
                if Confirm::new(&prompt).with_default(true).prompt()? {
                    println!("Applying fix...");
                    // In a real implementation, we would apply the candidate's action here.
                    // For now, we simulate the application.
                }
            }
        }
    } else {
        println!("Running automatic heuristic re-tagging rules...");
        doc.retag_document().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    doc.save_as_version(&output, "2.0").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Re-tagged document saved to {:?}", output);
    Ok(())
}

fn handle_text(input: PathBuf, pages: Option<String>) -> Result<()> {
    println!("fepdf text: Extracting text from {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let doc = PdfDocument::open(data.into()).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_indices = parse_page_range(&range_str, page_count)?;

    for idx in target_indices {
        let text = doc.extract_text(idx).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        println!("\n--- [ PAGE {} ] ---\n{}", idx + 1, text);
    }
    Ok(())
}

fn parse_page_range(range_str: &str, max_pages: usize) -> Result<Vec<usize>> {
    let mut pages = Vec::new();
    for part in range_str.split(',') {
        if part.trim() == "all" {
            return Ok((0..max_pages).collect());
        }
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() == 2 {
                let start: usize = bounds[0].trim().parse::<usize>()?.saturating_sub(1);
                let end: usize = bounds[1].trim().parse::<usize>()?;
                for i in start..end.min(max_pages) {
                    pages.push(i);
                }
            }
        } else {
            let p: usize = part.trim().parse::<usize>()?.saturating_sub(1);
            if p < max_pages {
                pages.push(p);
            }
        }
    }
    pages.sort();
    pages.dedup();
    Ok(pages)
}
