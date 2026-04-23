#!/usr/bin/env python3
import json
import subprocess
import sys

# Whitelist of allowed licenses (Permissive or weak copyleft)
ALLOWED_LICENSES = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "BSD-1-Clause",
    "ISC",
    "CC0-1.0",
    "Unlicense",
    "BSL-1.0",
    "Zlib",
    "Unicode-3.0",
    "MPL-2.0",
    "MIT-0",
    "Apache-2.0 WITH LLVM-exception",
]

# Explicitly forbidden licenses (Strong copyleft)
FORBIDDEN_KEYWORDS = ["GPL", "AGPL"]

def is_safe(license_str):
    if not license_str:
        return False
    
    # Normalize potential separators
    normalized = license_str.replace("/", " OR ").replace("|", " OR ").replace(" AND ", " OR ")
    # Split into potential individual licenses
    parts = normalized.split(" OR ")
    
    # If ANY part of a dual/triple license is in the whitelist, we consider it safe
    for part in parts:
        clean_part = part.strip().strip("()")
        if clean_part in ALLOWED_LICENSES:
            return True
            
    return False

def audit():
    print("--- Starting License Audit ---")
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1"],
            capture_output=True,
            text=True,
            check=True
        )
    except subprocess.CalledProcessError as e:
        print(f"Error running cargo metadata: {e}")
        sys.exit(1)

    data = json.loads(result.stdout)
    packages = data.get("packages", [])
    
    violations = []
    
    for pkg in packages:
        name = pkg["name"]
        version = pkg["version"]
        license_str = pkg.get("license")
        
        if not is_safe(license_str):
            violations.append((name, version, license_str))
            
    if violations:
        print(f"FAILED: Found {len(violations)} packages with non-whitelisted or conflicting licenses:")
        for name, ver, lic in violations:
            print(f"  - {name} ({ver}): {lic}")
        return False
    else:
        print("PASS: All dependencies comply with the license policy.")
        return True

if __name__ == "__main__":
    if not audit():
        sys.exit(1)
    sys.exit(0)
