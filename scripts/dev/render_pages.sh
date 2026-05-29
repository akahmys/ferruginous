#!/bin/bash
# Render pages 1 to 5 of all PDFs in samples directory to out/artifacts.

set -e

mkdir -p out/artifacts

echo "=== Starting PDF Page Rendering to PNG ==="

for pdf in samples/*.pdf; do
    if [ ! -f "$pdf" ]; then
        continue
    fi

    base=$(basename "$pdf" .pdf)
    echo ""
    echo "📄 Processing $pdf..."

    # Get page count using analyze info
    info_output=$(./target/debug/fepdf analyze info "$pdf")
    page_count=$(echo "$info_output" | grep -E "Pages:[[:space:]]*[0-9]+" | awk '{print $2}')

    if [ -z "$page_count" ]; then
        echo "⚠️  Could not extract page count for $pdf. Defaulting to 5."
        max_page=5
    else
        echo "📋 Total Pages: $page_count"
        if [ "$page_count" -lt 5 ]; then
            max_page=$page_count
        else
            max_page=5
        fi
    fi

    for ((page=1; page<=max_page; page++)); do
        output_file="out/artifacts/${base}_page_${page}.png"
        echo "  🎨 Rendering page $page of $max_page to $output_file..."
        
        # Run fepdf produce render
        if ./target/debug/fepdf produce render "$pdf" "$output_file" --page "$page"; then
            echo "    ✅ Success"
        else
            echo "    ❌ Failed to render page $page"
        fi
    done
done

echo ""
echo "=== All completed! Output saved to out/artifacts/ ==="
ls -l out/artifacts
