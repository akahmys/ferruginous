---
description: プロジェクト規約、スキル、ワークフローの全体的な再編と整理
---

# [Workflow] 規約リファクタリング (Refactor Conventions)

プロジェクトの成長に伴う規約の冗長化を排除し、SSoT を再構築せよ。

## 手順

// turbo
1. **全体監査**: `GEMINI.md`、`protocols/`、`skills/`、`workflows/` 全域を走査し、冗長な記述やリンク切れを特定せよ。
// turbo
2. **基盤刷新**: `GEMINI.md`（憲章）と `PLANNING_PROTOCOL.md`（管理）の役割を分離し、役分担を明確化せよ。
// turbo
3. **記述の統一**: `RELIABLE_RUST_15.md` 等の各規約を「規約・目的・判定基準」の標準フォーマットへ統一し、曖昧さを排除せよ。
// turbo
4. **自動化の拡張**: 全ワークフローに `// turbo` アノテーションを付与し、かつリンクパスを最新の配置に修復せよ。
// turbo
5. **組織化**: 不要な一時ファイルの削除、および迷い込んだ仕様書を `specs/` 配下へ統合せよ。

// turbo
6. **同期確認**: [sync_docs](sync_docs.md) を実行し、全ドキュメントの整合性を最終確認せよ。

## 完了要件

- 階層構造（Constitution -> Protocols -> Specifications）の維持
- `verify_compliance.sh` による等価な品質担保
- 全リンクの正常動作
