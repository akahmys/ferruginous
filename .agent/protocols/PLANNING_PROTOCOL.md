# 戦略的プランニング・プロトコル (Planning Protocol)

> [!IMPORTANT]
> 単一案への固執を排し、多角的な解決策を検討せよ。全ファイルを役割に基づき基準化し、規約を自律的に洗練（Self-Refinement）せよ。

## 1. SSoT 役割定義 (SSoT Role Standard)

プロジェクトの全構成要素の「役割」と「性質」を定義する。改定・追加時は本表に基づきターゲットを特定せよ。

| 分類 | ファイル/ディレクトリ | 役割・SSoT 定義 | 判定基準 (Criteria) |
| :--- | :--- | :--- | :--- |
| **憲章** | `.agent/GEMINI.md` | **Constitution**: AI の操作原則。 | 全プロトコルの絶対的優先順位 |
| **規約** | `.agent/protocols/*.md` | **Protocols**: 安全性、品質、設計基準。 | 各ファイル内の「判定基準」セクション |
| **スキル** | `.agent/skills/*.md` | **Skills**: 高度な自動化手順の定義（摩擦分析等）。 | 出力の有用性と規約への還元率 |
| **手順** | `.agent/workflows/*.md` | **Workflows**: 定型作業の自動化（// turbo）。 | 実行エラー・警告の有無 |
| **仕様** | `specs/*.md` | **Specifications**: 設計・構造の正典。 | ISO 32000-2 との整合性 |
| **証跡** | `task.md` / `ROADMAP.md` | **Artifacts**: 進捗と完了の記録。 | 毎ターンの更新 |
| **計画** | `implementation_plan.md` | **Artifacts**: 意思決定と合意の記録。 | 承認の有無 |
| **証明** | `walkthrough.md` | **Artifacts**: 実装と検証のエビデンス。 | テスト・動画・キャプチャの有無 |

---

## 2. 計画策定サイクル (Phase 1: Research & Plan)

- **規約**: 既存コード、仕様書 (`specs/`)、Arlington Model を調査し、`implementation_plan.md` でアプローチを明文化せよ。
- **目的**: 曖昧な実装を排除し、リスクとトレードオフを事前に可視化する。
- **判定**: `implementation_plan.md` が作成され、ユーザーの承認が得られていること。

---

## 3. 実行と検証 (Phase 2 & 3: Execute & Verify)

- **規約**: `task.md` を最小単位で更新・消化。ロジック記述前にハーネス（テスト）を構築せよ。
- **目的**: 開発状況を透明化し、RR-15 違反を未然に防ぐ。
- **判定**: `task.md` の `[x]` 更新、および `./scripts/verify_compliance.sh` の完遂。

---

## 4. 規約の自律洗練 (Role-Aware Self-Refinement)

摩擦（Friction: エラー、指摘、非効率）を検知した際は、直ちに [analyze_friction](../skills/analyze_friction.md) スキルを実行し、以下の分類に基づき適切な規約を洗練せよ。

- **判断・行動ミス** $\rightarrow$ `GEMINI.md` へ。
- **実装・品質不備** $\rightarrow$ `RELIABLE_RUST_15.md` へ。
- **プロセス・同期不全** $\rightarrow$ `PLANNING_PROTOCOL.md` へ。
- **UI 整合性・UX 不備** $\rightarrow$ `UI_DESIGN_PROTOCOL.md` へ。
- **検証漏れ・監査不全** $\rightarrow$ `COMPLIANCE_STRATEGY.md` へ。

---

## 5. ポータビリティと匿名性の確保 (Portability & Privacy)

- **規約**: 絶対パス（`file:///Users/...` 等）の使用を厳禁し、ワークスペース内相対パスを強制せよ。
- **目的**: プロジェクトの移動や環境変化への耐性（Portability）、および公開時のプライバシー保護を確実に実施する。
- **判定基準**: ドキュメント内から `/Users/` 等の環境依存文字列が排除されていること。

## 6. 同期と完了報告 (Phase 4: Audit & Finalize)

定期的に [sync_docs](../workflows/sync_docs.md) を実行し、全ドキュメントを「今、起きている現実」に同期せよ。

- **Sync & Audit**: ターン末に `scripts/verify_compliance.sh` を実行し、Clippy 警告ゼロ及び規約適合を確認。
- **Walkthrough**: `walkthrough.md` にエビデンス（テスト出力、画像等）を記録。
- **Document Sync**: [sync_docs](../workflows/sync_docs.md) 実行。全文書を実態に強制同期。

---

## 7. 言語と記述スタイル (Language & Style)

- **実施計画書の言語**: `implementation_plan.md` は、意思決定の透明性を高めるため、**必ず日本語で記述**せよ。
- **技術用語の扱い**: Rust の型名、PDF 規格の用語、メソッド名等は英語のまま保持し、説明文を日本語で行え。
- **トーン**: 曖昧さを排した、事実と論理に基づく「断定的・命令調」の記述を維持せよ。

---

## 8. バージョン管理規約 (Version Control Protocol)

Ferruginous プロジェクトの整合性を保つため、以下の Git 運用を遵守せよ。

- **ブランチ戦略**: Trunk-based Development を採用。機能開発は `feat/`、修正は `fix/` プレフィックスの短命ブランチで行え。
- **コミットメッセージ**: `[Rule-ID] <type>: <description>` の形式を強制せよ。
    - **Rule-ID**: RR-15 のルール ID、または仕様書の節番号を示す。
    - **Type**: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `ci`, `chore`。
    - **例**: `[RR-15-2] fix: 境界条件における unwrap を Result へ置換`
- **マージ条件**: `scripts/verify_compliance.sh` および `scripts/verify_secrets.sh` がパスしていること。
- **秘密情報**: API キー等の秘密情報は絶対に含めてはならない。`.git/hooks/pre-push` による強制チェックを有効化せよ。

