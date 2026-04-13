#!/bin/bash
# scripts/verify_visuals.sh
# 視覚的回帰テストの実行スクリプト

set -e

# PATH の設定（Cargo が見つからない問題への対策）
export PATH="$HOME/.cargo/bin:$PATH"

echo "=== Visual Regression Audit Starting ==="

# テストターゲットの実行
# UPDATE_BASELINES=1 を指定すると基線を更新できる
if [ "$1" == "--update" ]; then
    echo "Mode: Update Baselines"
    env UPDATE_BASELINES=1 cargo test --package ferruginous-render --test visual_regression -- --nocapture
else
    echo "Mode: Verification"
    cargo test --package ferruginous-render --test visual_regression -- --nocapture
fi

echo "=== Visual Regression Audit Completed Successfully ==="
