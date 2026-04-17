#!/bin/bash

# 使い方: ./capture_window.sh "ウィンドウ名の一部" "保存先パス"
# 例: ./capture_window.sh "Ferruginous" "./screenshot.png"

WINDOW_TITLE_PART=$1
OUTPUT_FILE=${2:-"./window_capture.png"}

if [ -z "$WINDOW_TITLE_PART" ]; then
    echo "Usage: $0 <window_title_part> [output_file]"
    exit 1
fi

# AppleScriptでウィンドウIDを取得
WINDOW_ID=$(osascript -e "
tell application \"System Events\"
    set proc_list to every process whose background only is false
    repeat with proc in proc_list
        try
            repeat with win in windows of proc
                if title of win contains \"$WINDOW_TITLE_PART\" then
                    -- ウィンドウを最前面に移動（キャプチャ精度向上のため）
                    set frontmost of proc to true
                    return id of win
                end if
            end repeat
        end try
    end repeat
end tell
")

if [ -z "$WINDOW_ID" ] || [ "$WINDOW_ID" == "null" ]; then
    echo "Error: Window containing '$WINDOW_TITLE_PART' not found."
    exit 1
fi

# 指定したウィンドウIDのみをキャプチャ (-l フラグ)
# -x はシャッター音無効
screencapture -x -l "$WINDOW_ID" "$OUTPUT_FILE"

echo "Success: Captured window ID $WINDOW_ID to $OUTPUT_FILE"
