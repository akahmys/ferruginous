use ferruginous_sdk::PdfDocument;
use std::path::{Path, PathBuf};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples_dir = Path::new("samples");
    if !samples_dir.exists() {
        println!("Samples directory not found.");
        return Ok(());
    }

    let mut files = Vec::new();
    collect_pdfs(samples_dir, &mut files)?;

    println!("Found {} PDF files to validate.\n", files.len());

    let mut failures = 0;
    for file in files {
        print!("Validating {} ... ", file.display());
        match validate_pdf(&file) {
            Ok(_) => println!("OK"),
            Err(e) => {
                println!("FAILED: {}", e);
                failures += 1;
            }
        }
    }

    if failures > 0 {
        println!("\nTotal failures: {}", failures);
        std::process::exit(1);
    } else {
        println!("\nAll files validated successfully.");
    }

    Ok(())
}

fn collect_pdfs(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_pdfs(&path, files)?;
            } else if path.extension().map_or(false, |ext| ext == "pdf") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn validate_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    
    // 1. Load structure
    let doc = ferruginous_sdk::loader::load_document_structure(&data)?;
    
    // 2. Resolve Catalog
    let catalog = doc.catalog()?;
    
    // 3. Resolve Page Tree
    let _page_tree = catalog.page_tree()?;
    
    Ok(())
}
