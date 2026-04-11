---
description: RR-15 コンプライアンスの自動検証ワークフロー
---

# [Workflow] 整合性検証 (Verify Compliance)

[RR-15](../protocols/RELIABLE_RUST_15.md) 適合性を「スキル」と「スクリプト」の両面から監査し、絶対的な品質を維持せよ。

## 手順

// turbo
1. **AI監査**: [verify_rr15](../skills/verify_rr15.md) スキルに従い、現状のコードベースに論理的・構造的な違反がないかセルフチェックせよ。
2. **自動監査**: 以下を実行し、機械的な全項目 PASS を確認せよ。
// turbo
   - `./scripts/verify_compliance.sh` (規約適合性)
// turbo
   - `cargo clippy --pedantic` (警告ゼロ)
// turbo
   - `./scripts/msrv_check.sh` (互換性維持)
3. **成果物記録**: 監査結果と発見された課題、修正内容を `walkthrough.md` に記録せよ。

## 完了要件
- 警告・不整合が 1 件も残っていないこと
- AIによる論理監査において「改善の余地なし」と判断されること
- 修正が必要な場合は、機能追加を中断して修正を優先したこと
