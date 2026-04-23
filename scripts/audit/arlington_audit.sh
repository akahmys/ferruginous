#!/bin/bash
# Arlington PDF Model External Auditor Wrapper
# Usage: ./scripts/arlington_audit.sh <target.pdf>

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <target.pdf>"
    exit 1
fi

TARGET_PDF=$1
TSV_DIR="external/arlington/tsv/latest"
PYTHON_BIN="./.arlington-venv/bin/python3"
AUDITOR_SCRIPT="external/arlington/scripts/arlington.py"

echo "=== External Audit: Arlington PDF Model ==="
echo "Target: $TARGET_PDF"
echo "TSVs: $TSV_DIR"

if [ ! -f "$TARGET_PDF" ]; then
    echo "Error: Target PDF not found at $TARGET_PDF"
    exit 1
fi

# Run the validation
$PYTHON_BIN $AUDITOR_SCRIPT --tsvdir $TSV_DIR --pdf "$TARGET_PDF" --validate

echo "=== External Audit Complete ==="
