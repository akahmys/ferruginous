# 戦略的プランニング・プロトコル (Planning Protocol)

> [!IMPORTANT]
> 単一案への固執を排し、多角的な解決策を検討せよ。全ファイルを役割に基づき基準化し、規約を自律的に洗練（Self-Refinement）せよ。

## 1. SSoT 役割定義 (SSoT Role Standard)

プロジェクトの全構成要素の「役割」と「性質」を定義する。改定・追加時は本表に基づきターゲットを特定せよ。

| 分類 | ファイル/ディレクトリ | 役割・SSoT 定義 | 判定基準 (Criteria) |
| :--- | :--- | :--- | :--- |
| **憲章** | `.agent/GEMINI.md` | **Constitution**: AI の操作原則とモード定義。 | 全プロトコルの絶対的優先順位 |
| **規約** | `.agent/protocols/*.md` | **Protocols**: 安全性、品質、設計基準。 | 各ファイル内の「判定基準」セクション |
| **スキル** | `.agent/skills/*.md` | **Skills**: 高度な自動化手順の定義（摩擦分析等）。 | 出力の有用性と規約への還元率 |
| **手順** | `.agent/workflows/*.md` | **Workflows**: 定型作業の自動化（// turbo）。 | 実行エラー・警告の有無 |
| **仕様** | `specs/*.md` | **Specifications**: 設計・構造の正典。 | ISO 32000-2 との整合性 |
| **作業記録** | `.agent/session/task.md` | **WAL**: 作業計画と進捗の即時記録。 | 作業開始前に Plan が記述されていること |
| **引き継ぎ** | `.agent/session/handoff.md` | **Handoff**: セッション間の状態伝達。 | セッション終了時に更新されていること |
| **失敗記録** | `.agent/session/regression_log.md` | **Anti-Patterns**: 失敗と学びの永続記録。 | 追記のみ。セッション終了時に更新 |
| **進捗** | `ROADMAP.md` | **Milestones**: マイルストーンの完了記録。 | 完了ゲート通過 + ユーザー承認 |
| **計画** | `.agent/session/implementation_plan.md` | **Artifacts**: 意思決定と合意の記録。 | 承認の有無 |
| **証明** | `.agent/session/walkthrough.md` | **Artifacts**: 実装と検証のエビデンス。 | テスト・動画・キャプチャの有無 |

---

## 2. 計画策定サイクル (Phase 1: Research & Plan)

- **規約**: 既存コード、仕様書 (`specs/`)、Arlington Model を調査し、`implementation_plan.md` でアプローチを明文化せよ。
- **目的**: 曖昧な実装を排除し、リスクとトレードオフを事前に可視化する。
- **判定**: `implementation_plan.md` が作成され、ユーザーの承認が得られていること。

---

## 3. セッション管理と実行 (Session Management & Execution)

### 3.1. セッション開始手順 [MUST]

以下の手順をセッション開始時に必ず実行せよ。省略は禁止。

1. `.agent/GEMINI.md` を読み、原則とモード定義を確認する
2. `.agent/session/handoff.md` を読み、前セッションの状態を把握する
3. `.agent/session/regression_log.md` を読み、過去の失敗を確認する
4. 作業モード（Build / Fix）を判定し、人間に宣言して承認を得る
5. このセッションのスコープを宣言する
6. `.agent/session/task.md` に計画を書く（WAL: 実行前に意図を記録）

### 3.2. 作業中の WAL 運用 [MUST]

- **規約**: `.agent/session/task.md` を WAL パターンで運用せよ。**各ステップの着手前に意図を記録し、完了後に結果を記録する。**
- **目的**: AI の途中停止や文脈喪失に備え、作業意図を永続化する。
- **判定**: `task.md` の各ステップに着手前の記述と完了後の結果が記録されていること。

### 3.3. セッション終了手順 [MUST]

1. 完了ゲートの判定（モードに応じたプロトコルに従う）
2. `.agent/session/handoff.md` を更新する（未解決問題の有無にかかわらず必須）
3. `.agent/session/regression_log.md` を更新する（失敗や学びがあった場合）
4. 完了していない作業を正直に「未完了」と報告する

---

## 4. 規約の自律洗練 (Role-Aware Self-Refinement)

摩擦（Friction: エラー、指摘、非効率）を検知した際は、直ちに [analyze_friction](../skills/analyze_friction.md) スキルを実行し、以下の分類に基づき適切な規約を洗練せよ。

- **判断・行動ミス** $\rightarrow$ `GEMINI.md` へ。
- **実装・品質不備** $\rightarrow$ `RELIABLE_RUST_15.md` へ。
- **プロセス・同期不全** $\rightarrow$ `PLANNING_PROTOCOL.md` へ。
- **修正プロセスの失敗**（リグレッション、誤診断等） $\rightarrow$ `FIX_PROTOCOL.md` へ。
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
- **Walkthrough**: `.agent/session/walkthrough.md` にエビデンス（テスト出力、画像等）を記録。
- **Document Sync**: [sync_docs](../workflows/sync_docs.md) 実行。全文書を実態に強制同期。

---

## 7. 言語と記述スタイル (Language & Style)

- **実施計画書の言語**: `implementation_plan.md` は、意思決定の透明性を高めるため、**必ず日本語で記述**せよ。
- **技術用語の扱い**: Rust の型名、PDF 規格の用語、メソッド名等は英語のまま保持し、説明文を日本語で行え。
- **トーン**: 事実と論理に基づく記述を維持せよ。ただし、**Fix モードでは不確実性を明示する表現**（「Xの可能性がある」「未検証」等）を積極的に使い、虚偽の確信を排除すること。

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

