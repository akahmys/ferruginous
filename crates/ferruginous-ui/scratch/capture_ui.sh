#!/bin/bash
# GUI ウィンドウのスクリーンショットを撮影し、アーティファクトディレクトリに保存する
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUT_DIR="/Users/jun/.gemini/antigravity/brain/b7be1f2a-14bb-42bf-9693-53645c92e0dd"
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
