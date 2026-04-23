#!/bin/bash
# safety_vitals.sh (DEPRECATED)
# This script is deprecated. Redirecting to verify_compliance.sh.

echo "WARNING: safety_vitals.sh is deprecated. Use scripts/verify_compliance.sh instead."
echo "Redirecting to scripts/verify_compliance.sh..."
exec "$(dirname "$0")/verify_compliance.sh" "$@"
