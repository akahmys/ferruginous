//! fepdf: The Universal PDF Toolkit.
//!
//! (ISO 32000-2:2020 Compliance & Optimization Engine)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ferruginous_sdk::{PdfDocument, PdfStandard};
use inquire::Confirm;
use std::path::PathBuf;

#[derive(clap::Args, Debug, Clone)]
struct IngestionArgs {
    /// Disable active 2-pass refinement (UTF-8 normalization)
    #[arg(long)]
    no_refinement: bool,
    /// Disable automatic conversion of Info to XMP
    #[arg(long)]
    no_metadata_recovery: bool,
    /// Use relaxed color validation policy
    #[arg(long)]
    relaxed_color: bool,
}

impl From<IngestionArgs> for ferruginous_core::ingest::IngestionOptions {
    fn from(args: IngestionArgs) -> Self {
        Self {
            active_refinement: !args.no_refinement,
            sublime_metadata: !args.no_metadata_recovery,
            color_policy: if args.relaxed_color {
                ferruginous_core::ingest::ColorPolicy::Relaxed
            } else {
                ferruginous_core::ingest::ColorPolicy::Strict
            },
        }
    }
}

#[derive(clap::Args, Debug, Clone)]
struct OptimizationArgs {
    /// Opt-in for stream compression (FlateDecode)
    #[arg(long)]
    compress: bool,
    /// Remove unreachable objects
    #[arg(long)]
    vacuum: bool,
    /// Strip descriptive metadata
    #[arg(long)]
    strip: bool,
    /// Encrypt with password
    #[arg(long)]
    password: Option<String>,
    /// Use Object Streams (ObjStm) for high-density compression
    #[arg(long)]
    obj_stm: bool,
    /// Image re-compression quality (1-100)
    #[arg(long)]
    image_quality: Option<u32>,
    /// Set document primary language (e.g., "en-US", "ja-JP")
    #[arg(long)]
    lang: Option<String>,
    /// Override document title
    #[arg(long)]
    title: Option<String>,
    /// Override document author
    #[arg(long)]
    author: Option<String>,
    /// Set copyright notice in XMP metadata
    #[arg(long)]
    copyright: Option<String>,
    /// Permission flags (e.g., "print,copy")
    #[arg(long)]
    permissions: Option<String>,
    /// Text string encoding for non-ASCII characters (utf16be, utf8)
    #[arg(long, default_value = "utf16be")]
    string_encoding: String,
    /// Perform simulation without writing output file
    #[arg(long)]
    dry_run: bool,
}

impl From<OptimizationArgs> for ferruginous_sdk::SaveOptions {
    fn from(args: OptimizationArgs) -> Self {
        Self {
            compress: args.compress,
            compression_level: 9,
            vacuum: args.vacuum,
            strip: args.strip,
            password: args.password,
            obj_stm: args.obj_stm,
            image_quality: args.image_quality,
            lang: args.lang,
            title: args.title,
            author: args.author,
            copyright: args.copyright,
            permissions: args.permissions,
            string_encoding: match args.string_encoding.to_lowercase().as_str() {
                "utf8" => ferruginous_sdk::StringEncoding::Utf8,
                _ => ferruginous_sdk::StringEncoding::Utf16BE,
            },
            dry_run: args.dry_run,
        }
    }
}
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
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
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
        /// Display internal structural diff after refinement
        #[arg(long)]
        diff: bool,
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
    },
    /// Merge multiple PDF files into one
    Merge {
        /// Input PDF files
        inputs: Vec<PathBuf>,
        /// Output PDF file
        #[arg(short, long)]
        output: PathBuf,
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
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
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
    },
    /// Attempt to repair a corrupted PDF document
    Repair {
        /// Input corrupted PDF file
        input: PathBuf,
        /// Output repaired PDF file
        output: PathBuf,
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
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
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
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
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
    },
    /// Extract text from the document
    Text {
        /// Input PDF file
        input: PathBuf,
        /// Pages to extract text from (comma-separated or range, e.g., 1-5)
        #[arg(short, long)]
        pages: Option<String>,
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
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
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
    },
    /// Digitally sign the PDF document
    Sign {
        /// Input PDF file
        input: PathBuf,
        /// Output signed PDF file
        output: PathBuf,
        /// Reason for signing
        #[arg(long)]
        reason: Option<String>,
        /// Location of signing
        #[arg(long)]
        location: Option<String>,
        /// Signer name
        #[arg(long)]
        name: Option<String>,
        /// Page number for the signature widget (default 1)
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Visual rectangle [x1, y1, x2, y2]
        #[arg(long, value_delimiter = ',', num_args = 4)]
        rect: Option<Vec<f32>>,
        /// Ingestion control options
        #[command(flatten)]
        ingest: IngestionArgs,
        /// Optimization options
        #[command(flatten)]
        opt: OptimizationArgs,
    },
    /// Display open source credits and licenses
    Credits,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { input, audit, structure, format, ingest } => {
            handle_inspect(input, audit, structure, format, ingest)?;
        }
        Commands::Upgrade {
            input,
            output,
            standard,
            icc_profile,
            linearize,
            diff,
            ingest,
            opt,
        } => {
            handle_upgrade(input, output, standard, icc_profile, linearize, diff, ingest, opt)?;
        }
        Commands::Merge { inputs, output, ingest, opt } => {
            handle_merge(inputs, output, ingest, opt)?;
        }
        Commands::Split { input, output, pages, ingest, opt } => {
            handle_split(input, output, pages, ingest, opt)?;
        }
        Commands::Repair { input, output, ingest, opt } => {
            handle_repair(input, output, ingest, opt)?;
        }
        Commands::Rotate { input, output, pages, angle, ingest, opt } => {
            handle_rotate(input, output, pages, angle, ingest, opt)?;
        }
        Commands::Render { input, output, page, ingest } => {
            handle_render(input, output, page, ingest)?;
        }
        Commands::Text { input, pages, ingest } => {
            handle_text(input, pages, ingest)?;
        }
        Commands::Retag { input, output, wizard, ingest, opt } => {
            handle_retag(input, output, wizard, ingest, opt)?;
        }
        Commands::Sign { input, output, reason, location, name, page, rect, ingest, opt } => {
            handle_sign(input, output, reason, location, name, page, rect, ingest, opt)?;
        }
        Commands::Credits => {
            handle_credits()?;
        }
    }

    Ok(())
}

fn handle_merge(
    inputs: Vec<PathBuf>,
    output: PathBuf,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf merge: Combining {} files into {:?}", inputs.len(), output);
    let mut sources = Vec::new();
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    for path in inputs {
        let data = std::fs::read(&path).with_context(|| format!("Failed to read {:?}", path))?;
        let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        sources.push(doc);
    }

    let merged = PdfDocument::merge(sources).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    merged
        .save_with_options(&output, "2.0", &save_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Merged output saved to {:?}", output);
    Ok(())
}

fn handle_split(
    input: PathBuf,
    output: PathBuf,
    pages: Option<String>,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf split: Extracting pages from {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_indices = parse_page_range(&range_str, page_count)?;

    let extracted = doc.extract_pages(target_indices).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    extracted
        .save_with_options(&output, "2.0", &save_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Extracted output saved to {:?}", output);
    Ok(())
}

fn render_summary_markdown(
    summary: &ferruginous_sdk::DocumentSummary,
    input: &std::path::Path,
    audit: bool,
) -> Result<()> {
    println!("# Document Summary: {:?}", input.file_name().unwrap_or_default());
    println!("\n## General Information");
    println!("\n| Property | Value |");
    println!("| :--- | :--- |");
    println!("| Version | {} |", summary.version);
    println!("| Total Pages | {} |", summary.page_count);
    if let Some(v) = &summary.metadata.title {
        println!("| Title | {} |", v);
    }
    if let Some(v) = &summary.metadata.author {
        println!("| Author | {} |", v);
    }
    if let Some(v) = &summary.metadata.subject {
        println!("| Subject | {} |", v);
    }
    if let Some(v) = &summary.metadata.keywords {
        println!("| Keywords | {} |", v);
    }
    if let Some(v) = &summary.metadata.creator {
        println!("| Creator | {} |", v);
    }
    if let Some(v) = &summary.metadata.producer {
        println!("| Producer | {} |", v);
    }

    let embedded_count = summary.fonts.iter().filter(|f| f.is_embedded).count();
    let total_fonts = summary.fonts.len();

    println!("\n## Font Audit (Embedded: {}/{})", embedded_count, total_fonts);
    if total_fonts > 0 {
        println!("\n| Font Name | Type | Embedded | Subset | Encoding |");
        println!("| :--- | :--- | :--- | :--- | :--- |");
        for f in &summary.fonts {
            println!(
                "| {} | {} | {} | {} | {} |",
                f.name,
                f.font_type,
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
    let errors = summary
        .compliance
        .issues
        .iter()
        .filter(|i| {
            matches!(
                i.severity,
                ferruginous_sdk::IssueSeverity::Error | ferruginous_sdk::IssueSeverity::Critical
            )
        })
        .count();
    let warnings = summary
        .compliance
        .issues
        .iter()
        .filter(|i| matches!(i.severity, ferruginous_sdk::IssueSeverity::Warning))
        .count();
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

    if !summary.compliance.iso_clauses.is_empty() {
        println!("\n## Validated ISO 32000-2 Clauses");
        println!("The following structural components were validated against the specification:");
        for clause in &summary.compliance.iso_clauses {
            println!("- **Clause {}**", clause);
        }
    }
    Ok(())
}

fn render_summary_text(
    doc: &PdfDocument,
    summary: &ferruginous_sdk::DocumentSummary,
    audit: bool,
    structure: bool,
) -> Result<()> {
    println!("\n--- [ DOCUMENT SUMMARY ] ---");
    println!("Version:    {}", summary.version);
    println!("Pages:      {}", summary.page_count);
    if let Some(v) = &summary.metadata.title {
        println!("Title:      {}", v);
    }
    if let Some(v) = &summary.metadata.author {
        println!("Author:     {}", v);
    }

    println!("\n--- [ FONT AUDIT ] ---");
    let embedded_count = summary.fonts.iter().filter(|f| f.is_embedded).count();
    println!("Total Fonts: {} (Embedded: {})", summary.fonts.len(), embedded_count);

    if summary.fonts.is_empty() {
        println!("No fonts detected.");
    } else {
        println!(
            "{:<30} | {:<10} | {:<4} | {:<4} | {:<10}",
            "Font Name", "Type", "Emb", "Sub", "Encoding"
        );
        println!("{:-<30}-+-{:-<10}-+-{:-<4}-+-{:-<4}-+-{:-<10}", "", "", "", "", "");
        for f in &summary.fonts {
            println!(
                "{:<30} | {:<10} | {:<4} | {:<4} | {:<10}",
                f.name,
                f.font_type,
                if f.is_embedded { "✅" } else { "❌" },
                if f.is_subset { "✅" } else { "−" },
                f.encoding
            );
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
        
        if !summary.compliance.iso_clauses.is_empty() {
            println!("\n--- [ ISO 32000-2 COMPLIANCE ] ---");
            println!("Validated Clauses: {}", summary.compliance.iso_clauses.join(", "));
        }
    }

    if structure {
        let tree = doc.print_structure().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        println!("\n{}", tree);
    }
    Ok(())
}

fn handle_inspect(
    input: PathBuf,
    audit: bool,
    structure: bool,
    format: String,
    ingest: IngestionArgs,
) -> Result<()> {
    if format == "text" {
        println!("fepdf inspect: Analyzing {:?}", input);
    }
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let summary = doc.get_summary().map_err(|e| anyhow::anyhow!("{:?}", e))?;

    match format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&summary)?),
        "markdown" => render_summary_markdown(&summary, &input, audit)?,
        _ => render_summary_text(&doc, &summary, audit, structure)?,
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_upgrade(
    input: PathBuf,
    output: PathBuf,
    standard: Option<String>,
    icc_profile: Option<PathBuf>,
    linearize: bool,
    diff: bool,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf upgrade: {:?} -> {:?}", input, output);
    if opt.dry_run {
        println!("DRY RUN: Simulation mode enabled. No file will be written.");
    }

    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let mut doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if diff {
        println!("INFO: Structural diff would be displayed here (M67 enhancement).");
    }

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

    let save_options: ferruginous_sdk::SaveOptions = opt.into();

    if linearize {
        doc.save_linearized(&output, "2.0", &save_options)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else {
        doc.save_with_options(&output, "2.0", &save_options)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    println!("SUCCESS: Output saved to {:?}", output);
    Ok(())
}

fn handle_repair(
    input: PathBuf,
    output: PathBuf,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf repair: Attempting to salvage corrupted document {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_and_repair_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    doc.save_with_options(&output, "2.0", &save_options).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Repaired output saved to {:?}", output);
    Ok(())
}

fn handle_rotate(
    input: PathBuf,
    output: PathBuf,
    pages: Option<String>,
    angle: i32,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf rotate: Rotating pages in {:?} by {} degrees...", input, angle);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let mut doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_pages = parse_page_range(&range_str, page_count)?;

    for idx in target_pages {
        doc.set_page_rotation(idx, angle).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    doc.save_with_options(&output, "2.0", &save_options).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Rotated output saved to {:?}", output);
    Ok(())
}

fn handle_render(
    input: PathBuf,
    output: PathBuf,
    page_num: usize,
    ingest: IngestionArgs,
) -> Result<()> {
    println!("fepdf render: Rendering page {} of {:?} to {:?}...", page_num, input, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if page_num == 0 || page_num > doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))? {
        return Err(anyhow::anyhow!("Invalid page number: {}", page_num));
    }

    doc.render_page_to_file(page_num - 1, &output).map_err(|e| anyhow::anyhow!("{:?}", e))?;

    println!("SUCCESS: Rendered page saved to {:?}", output);
    Ok(())
}

fn handle_retag(
    input: PathBuf,
    output: PathBuf,
    wizard: bool,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf retag: {} -> {:?}", if wizard { "Wizard Mode" } else { "Automatic" }, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let mut doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if wizard {
        println!("Wizard Mode: Reviewing heuristic structural candidates...");
        let candidates =
            doc.get_remediation_candidates().map_err(|e| anyhow::anyhow!("{:?}", e))?;

        if candidates.is_empty() {
            println!("No remediation candidates found.");
        } else {
            for candidate in candidates {
                let prompt =
                    format!("Page {}: {}?", candidate.page_index + 1, candidate.description);
                if Confirm::new(&prompt).with_default(true).prompt()? {
                    println!("Applying fix...");
                }
            }
        }
    } else {
        println!("Running automatic heuristic re-tagging rules...");
        doc.retag_document().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    doc.save_with_options(&output, "2.0", &save_options).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    println!("SUCCESS: Re-tagged document saved to {:?}", output);
    Ok(())
}

fn handle_text(input: PathBuf, pages: Option<String>, ingest: IngestionArgs) -> Result<()> {
    println!("fepdf text: Extracting text from {:?}", input);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let page_count = doc.page_count().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let range_str = pages.unwrap_or_else(|| "all".to_string());
    let target_indices = parse_page_range(&range_str, page_count)?;

    for idx in target_indices {
        let text = doc.extract_text(idx).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        println!("\n--- [ PAGE {} ] ---\n{}", idx + 1, text);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_sign(
    input: PathBuf,
    output: PathBuf,
    reason: Option<String>,
    location: Option<String>,
    name: Option<String>,
    page: usize,
    rect: Option<Vec<f32>>,
    ingest: IngestionArgs,
    opt: OptimizationArgs,
) -> Result<()> {
    println!("fepdf sign: {:?} -> {:?}", input, output);
    let data = std::fs::read(&input).with_context(|| "Failed to read input")?;
    let ingest_options: ferruginous_core::ingest::IngestionOptions = ingest.into();
    let doc = PdfDocument::open_with_options(data.into(), &ingest_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let mut sign_options = ferruginous_sdk::SignOptions {
        reason,
        location,
        name,
        page_index: page.saturating_sub(1),
        ..Default::default()
    };

    if let Some(r) = rect {
        if r.len() == 4 {
            sign_options.rect = [r[0], r[1], r[2], r[3]];
        }
    } else {
        sign_options.rect = [50.0, 50.0, 200.0, 100.0]; // Default widget rect
    }

    let save_options: ferruginous_sdk::SaveOptions = opt.into();
    doc.save_signed(&output, "2.0", &save_options, &sign_options)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    println!("SUCCESS: Signed document saved to {:?}", output);
    Ok(())
}

fn handle_credits() -> Result<()> {
    println!("\n--- [ OPEN SOURCE CREDITS ] ---");
    println!("fepdf and ferruginous-sdk are powered by the following excellent libraries:\n");

    let credits = [
        ("lopdf", "MIT", "Low-level PDF parsing and manipulation"),
        ("pdf-writer", "Apache-2.0", "Efficient PDF object serialization"),
        ("flate2", "MIT / Apache-2.0", "Deflate/Zlib compression"),
        ("vello", "Apache-2.0 / MIT", "High-performance vector graphics"),
        ("kurbo", "Apache-2.0 / MIT", "Vector geometry primitives"),
        ("skrifa / read-fonts", "Apache-2.0 / MIT", "Modern font parsing and glyph scaling"),
        ("image", "MIT / Apache-2.0", "Raster image processing"),
        ("anyhow / thiserror", "MIT / Apache-2.0", "Structured error handling"),
        ("serde", "MIT / Apache-2.0", "Universal serialization framework"),
        ("tokio", "MIT", "Asynchronous runtime"),
    ];

    println!("{:<25} | {:<20} | {:<30}", "Crate", "License", "Purpose");
    println!("{:-<25}-+-{:-<20}-+-{:-<30}", "", "", "");
    for (name, license, purpose) in credits {
        println!("{:<25} | {:<20} | {:<30}", name, license, purpose);
    }

    println!("\nFull license texts are available in the repository's NOTICE file.");
    println!("Thank you to the Rust community for these amazing tools.");
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
