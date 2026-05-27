#!/bin/bash
set -e

# Compile fepdf in release mode for maximum performance
echo "Building fepdf in release mode..."
cargo build --release -p fepdf

FEPDF="./target/release/fepdf"
SAMPLES_DIR="./samples"
OUT_DIR="./out/artifacts"

mkdir -p "$OUT_DIR"

echo "Starting batch PDF 2.0 upgrade and optimization..."

for pdf in "$SAMPLES_DIR"/*.pdf; do
    if [ ! -f "$pdf" ]; then
        continue
    fi
    filename=$(basename "$pdf")
    
    echo "--------------------------------------------------"
    echo "Upgrading: $filename"
    
    # Run upgrade command with all requested options
    "$FEPDF" produce upgrade "$pdf" "$OUT_DIR/$filename" \
        --linearize \
        --compress \
        --vacuum \
        --strip \
        --obj-stm \
        --string-encoding utf16be
        
    echo "Completed: $filename"
done

echo "--------------------------------------------------"
echo "Batch conversion finished successfully!"
echo "Files saved to $OUT_DIR"
ls -lh "$OUT_DIR"
