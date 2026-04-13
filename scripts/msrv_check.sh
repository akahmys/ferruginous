#!/bin/bash
# MSRV Consistency Check Script

# Sources of truth
ROOT_CARGO="Cargo.toml"
TOOLCHAIN=".rust-toolchain.toml"

# Extract expected version from root Cargo.toml
EXPECTED_VERSION=$(grep "rust-version =" "$ROOT_CARGO" | head -n 1 | cut -d '"' -f 2)

if [ -z "$EXPECTED_VERSION" ]; then
    echo "Error: Could not determine expected rust-version from $ROOT_CARGO"
    exit 1
fi

echo "Checking for MSRV consistency with version: $EXPECTED_VERSION"

# 1. Check .rust-toolchain.toml
TOOLCHAIN_VERSION=$(grep "channel =" "$TOOLCHAIN" | cut -d '"' -f 2)
if [[ "$TOOLCHAIN_VERSION" != "$EXPECTED_VERSION" ]]; then
    echo "Error: $TOOLCHAIN version ($TOOLCHAIN_VERSION) does not match $ROOT_CARGO ($EXPECTED_VERSION)"
    exit 1
fi

# 2. Check for occurrences of the old MSRV (1.85.0)
OLD_MSRV="1.85.0"
echo "Searching for obsolete version: $OLD_MSRV"
MAPPINGS=$(grep -rn "$OLD_MSRV" . \
    --exclude-dir=.git \
    --exclude-dir=target \
    --exclude-dir=.gemini \
    --exclude-dir=.agent \
    --exclude-dir=node_modules \
    --exclude="*.pdf" \
    --exclude="*.log" \
    --exclude="Cargo.lock" \
    --exclude="msrv_check.sh" \
    --exclude="*.txt")

if [ -n "$MAPPINGS" ]; then
    echo "Error: Obsolete MSRV ($OLD_MSRV) found in the following files:"
    echo "$MAPPINGS"
    exit 1
fi

# 3. Check for any version mismatch in other Cargo.toml files
CARGO_TOMLS=$(find . -name "Cargo.toml" -not -path "./target/*")
for file in $CARGO_TOMLS; do
    # Skip if it is the root Cargo.toml (already checked)
    if [[ "$file" == "./$ROOT_CARGO" ]] || [[ "$file" == "$ROOT_CARGO" ]]; then
        continue
    fi
    # Skip if it uses workspace inheritance
    if grep -q "rust-version = { workspace = true }" "$file"; then
        continue
    fi
    
    version=$(grep "rust-version =" "$file" | cut -d '"' -f 2)
    if [ -n "$version" ] && [[ "$version" != "$EXPECTED_VERSION" ]]; then
        echo "Error: $file rust-version ($version) does not match $ROOT_CARGO ($EXPECTED_VERSION)"
        exit 1
    fi
done

echo "MSRV consistency check passed!"
exit 0
