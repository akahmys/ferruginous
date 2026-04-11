#!/bin/bash
# Reliable Rust-10 (RR-10) Audit Script - Version 2.0
# Enforcing high-integrity safety standards for AI-assisted development

SEARCH_PATH="${1:-.}"
ERROR=0

echo "=== RR-10 High-Integrity Safety Audit: $SEARCH_PATH ==="

# Rule 1: Function Line Limit (50 lines)
echo "--- Rule 1: Function Line Limit (50) ---"
find "$SEARCH_PATH" -name "*.rs" -not -path "*/target/*" | while read -r file; do
    awk '
    /^pub fn|^fn / { in_fn=1; fn_name=$0; fn_start=NR; } 
    in_fn && /^}/ { 
        lines=NR-fn_start; 
        if (lines > 50) { 
            print "ERROR: [" FILENAME "] Fn at L" fn_start " is " lines " lines: " fn_name;
            exit 1;
        } 
        in_fn=0;
    }
    ' "$file" || ERROR=1
done

# Rule 2: Panic Exclusion (No unwrap/expect)
echo "--- Rule 2: No unwrap/expect ---"
if grep -rE "\.(unwrap|expect)\(.*\)" "$SEARCH_PATH" --include="*.rs" --exclude-dir=target; then
    echo "ERROR: Forbidden panicking methods found (.unwrap() or .expect())."
    ERROR=1
fi

# Rule 3: No Unsafe
echo "--- Rule 3: No Unsafe ---"
if grep -r "unsafe {" "$SEARCH_PATH" --include="*.rs" --exclude-dir=target; then
    echo "ERROR: Forbidden 'unsafe' block found."
    ERROR=1
fi

# Rule 6: No recursion (Heuristic)
echo "--- Rule 6: No Recursion (Heuristic) ---"
find "$SEARCH_PATH" -name "*.rs" -not -path "*/target/*" | while read -r file; do
    awk '
    /^pub fn|^fn / { 
        for (i=1; i<=NF; i++) {
            if ($i == "fn") {
                fn_full = $(i+1);
                split(fn_full, a, /[<(]/); 
                fn_name = a[1];
                in_fn = 1;
                break;
            }
        }
    }
    in_fn {
        if ($0 ~ fn_name && $0 !~ "fn " fn_name && $0 !~ "^[[:space:]]*//") {
             print "WARNING: [" FILENAME "] Potential recursion at L" NR ": " fn_name;
        }
    }
    in_fn && /^}/ { in_fn = 0; }
    ' "$file"
done

if [ $ERROR -eq 1 ]; then
    echo "-----------------------------------"
    echo "RR-10 Safety Audit FAILED."
    exit 1
else
    echo "-----------------------------------"
    echo "RR-10 Safety Audit PASSED."
    exit 0
fi
