---
description: ドキュメント（ROADMAP, task, README, 設計書）の同期ワークフロー
---

# [Workflow] ドキュメント同期 (Sync Docs)

実装実態に基づき SSoT を維持せよ。

## 手順

1. **状況収集**: パースコード、テスト結果、Arlington 検証状況、および `crates/` 配下の各 crate のビルド状況を確認。
// turbo
2. **ROADMAP 同期**: [ROADMAP.md](../../ROADMAP.md) の進捗を最新のマイルストーン状況に更新。
// turbo
3. **task 同期**: `task.md` の完了項目をチェックし、次期タスクを明文化せよ。
// turbo
4. **仕様書更新**: 解析・描画ロジックの変更、および UI 統合の進展を [specs/](../../specs/) 配下の各設計書に反映。
// turbo
5. **README 同期**: 現在のフェーズ、RR-15 準拠状況を [README.md](../../README.md) に反映。

## 完了要件

- 全てのドキュメントが現在のコードベースの能力を正しく説明していること
- リンク切れがないこと
