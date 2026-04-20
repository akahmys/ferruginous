#!/bin/bash
# Ferruginous Secret Guardian v1.0
set -e

echo "=== Secret Scanning Starting ==="
ERROR=0

# Define patterns for sensitive information
PATTERNS=(
    "(A3T[A-Z0-9]|AKI""A|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16}" # AWS Access Key
    "-----BEGIN[ A-Z0-9]+PRIVATE KEY-----"                             # Private Keys
    "[0-9a-zA-Z]{32,45}.*(api_key|apikey|secret|token|password)"        # High entropy strings with key keywords
    "xox[bpgr]-[0-9]{12}-[0-9]{12}-[0-9]{12}-[a-z0-9]{32}"              # Slack Tokens
    "https://hooks.slack.com/services/T[A-Z0-9]{8}/B[A-Z0-9]{8}/[A-Za-z0-9]{24}" # Slack Webhooks
    "SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}"                          # SendGrid API Key
    "ey[A-Za-z0-9_=+-]+\.ey[A-Za-z0-9_=+-]+\.?[A-Za-z0-9._+/= -]*"        # JWT
)

# Files to skip
SKIP_FILES="\.gitignore\|\.git\|scripts/verify_secrets\.sh\|\.pdf$"

echo "Checking for secrets in staged and current files..."

for pattern in "${PATTERNS[@]}"; do
    # Search in all files except excluded ones
    if grep -rE -e "$pattern" . --exclude-dir=".git" --exclude-dir="target" --exclude-dir="external" --exclude-dir=".arlington-venv" --exclude="*.pdf" | grep -v "$SKIP_FILES" > /dev/null; then
        echo "  FAIL: Potential secret found matching pattern: $pattern"
        grep -rE -e "$pattern" . --exclude-dir=".git" --exclude-dir="target" --exclude-dir="external" --exclude-dir=".arlington-venv" --exclude="*.pdf" | grep -v "$SKIP_FILES"
        ERROR=1
    fi
done

# Personal Email Check (Heuristic)
# Note: Adjust these patterns if legitimate project emails are found
EMAIL_PATTERNS=(
    "[a-zA-Z0-9._%+-]+@gmail\.com"
    "[a-zA-Z0-9._%+-]+@yahoo\.co\.jp"
)

for email in "${EMAIL_PATTERNS[@]}"; do
    if grep -rE "$email" . --exclude-dir=".git" --exclude-dir="target" --exclude-dir="external" --exclude-dir=".arlington-venv" --exclude="*.pdf" | grep -v "$SKIP_FILES" | grep -v "README.md" > /dev/null; then
         echo "  WARN: Potential personal email found: $email"
         grep -rE "$email" . --exclude-dir=".git" --exclude-dir="target" --exclude-dir="external" --exclude-dir=".arlington-venv" --exclude="*.pdf" | grep -v "$SKIP_FILES" | grep -v "README.md"
         # Not necessarily an error, but a warning
    fi
done

if [ $ERROR -eq 1 ]; then
    echo "=== SECRET SCANNING FAILED ==="
    echo "CRITICAL: Secrets detected. Commit/Push aborted."
    exit 1
else
    echo "=== SECRET SCANNING PASSED ==="
    exit 0
fi
