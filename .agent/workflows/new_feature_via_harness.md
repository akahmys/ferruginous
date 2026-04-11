---
description: ハーネス主導の新機能開発ワークフロー
---

# [Workflow] ハーネス主導開発 (New Feature via Harness)

[HDD](../protocols/HDD_PROTOCOL.md) および [RR-15](../protocols/RELIABLE_RUST_15.md) に基づき、検証主導で実装を完遂せよ。

## 手順

// turbo
1. **準備**: 目的のブランチを作成し [initialize_harness](initialize_harness.md) を完遂。
// turbo
2. **実装**: `resolve_harness` スキルを実行し、RR-15 準拠でテストを Green 化。
// turbo
3. **セルフ監査**: [verify_compliance](verify_compliance.md) を実行し、静的解析・型安全性を担保。
// turbo
4. **ドキュメント反映**: [sync_docs](sync_docs.md) を実行し、ROADMAP と task を最新の実態に同期。


## 完了要件

- 全テストパス（単体・統合・負試験）
- Clippy (Pedantic) 警告ゼロ
- ISO Clause へのリンク記述
