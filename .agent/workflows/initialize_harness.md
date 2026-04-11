---
description: HDD プロトコルに基づく検証ハーネスの最速初期化
---

# [Workflow] ハーネス初期化 (Initialize Harness)

実装前の合格条件 (HDD) を定義せよ。

## 手順

1. **仕様特定**: [HDD_PROTOCOL](../protocols/HDD_PROTOCOL.md) および ISO 32000-2 から該当オブジェクト・Clause を特定。
2. **モデル参照**: Arlington PDF Model から期待されるフィールド定義、型、制約を確認。
// turbo
3. **ハーネス構築**: `scaffold_harness` スキルを実行し、`tests/` 配下に不合格テストを作成。
// turbo
4. **FAIL 確認**: `cargo test` を実行し、意図通りに FAIL することを確認せよ。

## 完了要件
- 仕様とのリンクがコードコメントに含まれていること
- テストが現状の「不全」を正しく証明していること
