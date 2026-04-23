#!/bin/bash
# verify_style.sh - Automated audit for imperative and concise protocols

TARGET_DIRS=".agent .gemini docs/PROTOCOLS"
# Previously checked for Japanese polite forms (desu/masu). 
# Now that we are unified to English, these are kept for reference or updated.
FORBIDDEN_PATTERNS=("こと$" "です$" "ます$" "である$" "ました$" "。" "だ。")

echo "--- Style Audit Starting ---"
FOUND_ERROR=0

for dir in $TARGET_DIRS; do
  if [ ! -d "$dir" ]; then continue; fi
  echo "Checking $dir..."
  for file in $(find "$dir" -name "*.md"); do
    for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
      if grep -q "$pattern" "$file"; then
        echo "FAIL: Style violation ($pattern) in $file"
        grep -n "$pattern" "$file"
        FOUND_ERROR=1
      fi
    done
  done
done

if [ $FOUND_ERROR -eq 0 ]; then
  echo "SUCCESS: All files follow the imperative/concise protocol."
  exit 0
else
  echo "FAILURE: Style violations found."
  exit 1
fi
