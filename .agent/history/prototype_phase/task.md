# 規約全体のリファクタリング・タスク (Convention Refactoring)

## 1. クリーンアップ & 組織化

- [x] `build_err.txt` の削除
- [x] `docs/omissions.md` を `specs/omissions.md` へ移動
- [x] `specs/omissions.md` の規約名を `RR-15` に統一

## 2. 憲章・プロトコル基盤の刷新

- [x] `GEMINI.md` の軽量化（インデックス化・憲章化）
- [x] `PLANNING_PROTOCOL.md` へのプロセス集約（SSoT 役割定義等）
- [x] `UI_DESIGN_PROTOCOL.md` の新設 (UI コンポーネント設計規約)

## 3. 品質・工程プロトコルの精緻化

- [x] `RELIABLE_RUST_15.md` のフォーマット統一
- [x] `HDD_PROTOCOL.md` の UI 統合対応・改善
- [x] `COMPLIANCE_STRATEGY.md` の実効性向上

## 4. スキル・ワークフローの連動強化

- [x] 全ワークフローのリンクパス修正
- [x] `sync_docs.md` のマルチ crate 対応
- [x] `// turbo` アノテーションによる自動化拡張

## 5. 最終検証

- [x] 整合性チェック（リンク切れ確認）
- [x] `Refine Rules` ワークフローを用いた自己監査
- [x] `Sync Docs` の実行
- [x] 絶対パスの排除と相対パスへの変換
- [x] 「ポータビリティの確保」を管理規約（PLANNING_PROTOCOL.md）に追記

---
> [!NOTE]
> 公開を想定し、GEMINI.md 等の規約ファイル内の絶対パスを相対パスへ置換しました。
