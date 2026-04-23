#!/bin/bash
set -e

# Directory for samples
SAMPLES_DIR="samples"
mkdir -p "$SAMPLES_DIR"

echo "Fetching sample PDFs for Phase 4 verification..."

# 1. Mozilla pdf.js test PDF (Traceable images and CIDFonts)
echo "Downloading PDF 1.7 Traceable sample..."
curl -L -o "$SAMPLES_DIR/pdf_js_test.pdf" "https://raw.githubusercontent.com/mozilla/pdf.js/main/test/pdfs/complex_viewer_test.pdf"

# 2. PDF 2.0 Sample (from PDF Association or similar reliable source)
echo "Downloading PDF 2.0 sample..."
curl -L -o "$SAMPLES_DIR/pdf20_sample.pdf" "https://github.com/pdf-association/pdf-issues/raw/master/test-files/PDF_2.0_test_file.pdf" || echo "Note: PDF 2.0 sample download failed, continuing..."

echo "Samples downloaded successfully to ./$SAMPLES_DIR/"
ls -lh "$SAMPLES_DIR"
