# 規約全体のリファクタリング計画 (Total Convention Refactoring)

プロジェクトの規模拡大に伴い、`GEMINI.md` と `PLANNING_PROTOCOL.md` の記述が重複し、SSoT（信頼できる唯一の情報源）が曖昧になっています。また、各プロトコルの記述レベルを統一し、AI がより正確に判断を下せるよう構造化します。

## ユーザーレビューが必要な項目

> [!IMPORTANT]
> **構造の変更**: `GEMINI.md` から具体的な手順を削り、インデックス機能に特化させます。手順の詳細は `PLANNING_PROTOCOL.md` に集約します。
> これにより、AI はまず `GEMINI.md` を読み、必要に応じて各プロトコルを参照する「二段構え」の探索を行うようになります。

## 提案される変更

### 1. 憲章の再定義: [GEMINI.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/GEMINI.md) [MODIFY]
- 手順の重複を排除し、プロジェクトの「北極星（絶対原則）」と「プロトコル・インデックス」に特化。
- AI の振る舞いの基本原則（命令調の維持、推測の排除）を強調。

---

### 2. プロセスの統合: [PLANNING_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/PLANNING_PROTOCOL.md) [MODIFY]
- `GEMINI.md` から移譲された「SSoT 役割定義」や「開発フェーズ」の完全な定義を格納。
- 自己洗練化プロトコルをより具体化。

---

### 3. 品質の再定義: [RELIABLE_RUST_15.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/RELIABLE_RUST_15.md) [MODIFY]
- 各項目の記述を「規約」「目的」「判定基準」で統一。
- 最近の Rust 1.8x 以降のベストプラクティスに基づき、表現を微調整（コア原則は維持）。

---

### 4. 検証・監査の強化: [HDD_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/HDD_PROTOCOL.md) & [COMPLIANCE_STRATEGY.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/COMPLIANCE_STRATEGY.md) [MODIFY]
- HDD サイクルをより現代的な（Vello 等の UI 統合を考慮した）ステップへ更新。
- Compliance Strategy に、現実的な監査スクリプトとの連携を明記。

---

### 5. スキルの精緻化: [.agent/skills/*.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/skills/) [MODIFY]
- **analyze_friction.md**: 摩擦分析のプロセスを 4 分類（判断・実装・プロセス・検証）に完全追従。
- **scaffold_harness.md** / **resolve_harness.md**: HDD プロトコルとの用語不一致を解消。

---

### 6. ワークフローの機能強化: [.agent/workflows/*.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/workflows/) [MODIFY]
- 全ワークフローのリンクパスを修正（相対パスから絶対パス/正しい相対パスへの整理）。
- **initialize_harness.md**: Arlington PDF Model の抽出手順を具体化。
- **sync_docs.md**: 複数プロジェクト（SDK/UI）に分かれている現状を反映し、各 crate の doc も同期対象に。
- **自動化促進**: 安全なリサーチ・同期コマンドに `// turbo` アノテーションを付与し、作業効率を向上。

---

### 7. ファイルの組織化とクリーンアップ [DELETE/MOVE]
- **[DELETE] build_err.txt**: 不要なビルドエラーログを削除。
- **[MOVE] omissions.md**: `docs/omissions.md` を `specs/omissions.md` へ移動し、仕様の一貫性を確保。
- **[Standardize]**: `omissions.md` 内の規約名称を `RR-10 v2` から最新の `RR-15` へ更新。

## 実行フェーズ

1. **Research**: 全プロトコル・スキル・ワークフローの依存関係を精査（完了済み）。
2. **Consolidation**: `GEMINI.md` と `PLANNING_PROTOCOL.md` を土台として整備。
3. **Standardization**: 全スキル・全プロトコルの記述レベルとフォーマットを統一。
4. **Workflow Optimization**: ワークフロー間の連動性を強化し、リンクミスを全廃。
5. **Verification**: 整合性チェックと `Refine Rules` を用いた自己検証。

## オープンな質問

- **追加プロトコル**: 「UI コンポーネント設計規約」などをこのタイミングで新設する必要はありますか？（現状は SDK 中心ですが、Phase 6 以降 UI 統合が加速しています）
- **書式の好み**: 現在の GitHub アラート（IMPORTANT/TIP 等）を多用するスタイルを継続して良いでしょうか？

## 検証計画

### 手動検証
- 各プロトコル間のクロスバインド（リンク）が正しいか確認。
- 階層構造が `Constitution -> Protocols -> Specifications` になっているか確認。
- `Refine Rules` ワークフローを実行し、新しい構造で正常に動作するか確認。
