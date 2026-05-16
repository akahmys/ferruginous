#!/bin/bash
SAMPLES_DIR="../../samples"
ARTIFACTS_DIR="../../out/artifacts"
FEPDF="../../target/debug/fepdf"

mkdir -p "$ARTIFACTS_DIR"

for pdf in "$SAMPLES_DIR"/*.pdf; do
    base=$(basename "$pdf" .pdf)
    
    echo "Processing $base (PNG)..."
    "$FEPDF" produce render "$pdf" "$ARTIFACTS_DIR/${base}_p1.png" --page 1
    
    echo "Processing $base (PDF 2.0)..."
    "$FEPDF" produce upgrade "$pdf" "$ARTIFACTS_DIR/${base}_v2.pdf" \
        --linearize --compress --vacuum --obj-stm --string-encoding utf16be
done
