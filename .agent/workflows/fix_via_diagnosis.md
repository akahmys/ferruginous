---
description: FIX_PROTOCOL に基づく不具合修正の自動化ワークフロー
---

# [Workflow] 診断主導の不具合修正 (Fix via Diagnosis)

修正は開発ではない。不具合の真因を特定し、最小限の変更で解決せよ。

## 手順

1. **セッション初期化**: [.agent/GEMINI.md](../../.agent/GEMINI.md) の始動時命令に従い、handoff と regression_log を読み込む。
// turbo
2. **現象の記述**: 現象を観察し、[.agent/session/task.md](../session/task.md) に現在の状況を正確に記録する。
// turbo
3. **過去の履歴確認**: [.agent/session/regression_log.md](../session/regression_log.md) を読み、過去の失敗パターンをリストアップする。
// turbo
4. **仮説の列挙**: なぜ起きているか、仮説を **3つ以上** task.md に列挙する。
// turbo
5. **診断の設計・実行**: プロダクションコードを変更せずに仮説を検証する。診断用のログ追加や新規テスト作成を行う。
// turbo
6. **最小変更の実装**: 最も確度の高い仮説に基づき、1箇所だけプロダクションコードを変更する。
// turbo
7. **効果検証**: 症状が解消されたことをテストまたは目視（UIのみ）で確認。
// turbo
8. **リグレッション確認**: `scripts/verify_compliance.sh` および全自動テストを実行し、新たな破壊がないことを実証。
// turbo
9. **記録と終了**: 結果を task.md, regression_log.md, handoff.md に記録し、セッションを終了する。

## 完了要件

- バグ再現テストが test suite に追加され、PASS していること
- 全既存テストが PASS していること
- 実証エビデンスが [.agent/session/walkthrough.md](../session/walkthrough.md) に記録されていること
- ユーザーによる最終承認が得られていること
