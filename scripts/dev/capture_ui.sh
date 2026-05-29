#!/bin/bash
# Capture a screenshot of the GUI window and save it to the artifact directory.
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUT_DIR="${OUT_DIR:-.}"
OUT_PATH="${OUT_DIR}/ui_verification_${TIMESTAMP}.png"

mkdir -p "${OUT_DIR}"
echo "Capturing screen to ${OUT_PATH}..."
# -x: mute sound
screencapture -x "${OUT_PATH}"

if [ -f "${OUT_PATH}" ]; then
    echo "Successfully captured: ${OUT_PATH}"
else
    echo "Failed to capture screenshot."
fi
