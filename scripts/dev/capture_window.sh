#!/bin/bash

# Usage: ./capture_window.sh "partial window title" "output path"
# Example: ./capture_window.sh "Ferruginous" "./screenshot.png"

WINDOW_TITLE_PART=$1
OUTPUT_FILE=${2:-"./window_capture.png"}

if [ -z "$WINDOW_TITLE_PART" ]; then
    echo "Usage: $0 <window_title_part> [output_file]"
    exit 1
fi

# Get Window ID via AppleScript
WINDOW_ID=$(osascript -e "
tell application \"System Events\"
    set proc_list to every process whose background only is false
    repeat with proc in proc_list
        try
            repeat with win in windows of proc
                if title of win contains \"$WINDOW_TITLE_PART\" then
                    -- Bring window to front (for better capture accuracy)
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

# Capture only the specified Window ID (-l flag)
# -x disables shutter sound
screencapture -x -l "$WINDOW_ID" "$OUTPUT_FILE"

echo "Success: Captured window ID $WINDOW_ID to $OUTPUT_FILE"
