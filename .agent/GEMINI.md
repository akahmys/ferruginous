# Ferruginous AI 開発憲章 (GEMINI.md)

> [!IMPORTANT]
> **AI 始動時命令**: 全作業の開始前に、以下を順に読み込みコンテキストを確立せよ。
> 1. 本ドキュメント（憲章）
> 2. `.agent/session/handoff.md`（前セッションからの引き継ぎ）
> 3. `.agent/session/regression_log.md`（過去の失敗記録）
> 4. `specs/` 配下の仕様書（該当する場合）
>
> **方針判断は人間に委ねよ。** 独断で大きな設計変更・方針転換を行ってはならない。

## 1. 開発哲学 (Core Philosophy)

Ferruginous は「妥協なき安全性」と「機械的証明」を基盤とする。
各セッションの AI は **記憶を持たない新しい契約開発者** である。この制約を自覚し、以下の原則を固守せよ。

### 原則 1: 推論より実証

人間の推論も AI の推論も誤る。**目視確認は実証ではない。**
自動テストの通過、またはリファレンス実装との数値比較をもって「実証」とせよ。

### 原則 2: 記憶の外部化

AI は自身の記憶を信頼してはならない。
セッションをまたいで必要な情報は **すべてプロジェクト内のファイルに書き出す** こと。
書き出されていない知識は、次のセッションでは **存在しない**。

### 原則 3: 正直な状態報告

ROADMAP、task.md 等のドキュメントは **検証済みの事実のみ** を反映せよ。
テストで実証されていない「完了」は記載してはならない。

---

## 2. 作業モード (Operating Modes)

作業の性質に応じて、適用するプロトコルを切り替えよ。
**セッション開始時にモードを判定し、人間に宣言して承認を得ること。**

### Build モード（新機能の開発）

仕様に基づき新しいコードを書く場合に適用する。

- **適用プロトコル**: [HDD_PROTOCOL](protocols/HDD_PROTOCOL.md)
- **思考**: 仕様を読む → テストを書く → 実装する → テストが通る
- **スコープ**: 機能単位で広くてよい

### Fix モード（不具合の修正）

既存コードの不具合を調査・修正する場合に適用する。

- **適用プロトコル**: [FIX_PROTOCOL](protocols/FIX_PROTOCOL.md)
- **思考**: 観察する → 仮説を立てる → 診断する → 最小限の変更を加える
- **スコープ**: 検証可能な最小単位に限定する

---

## 3. 規約インデックス (Protocols Index)

実行する作業の性質に基づき、以下の各プロトコルを適宜参照・適用せよ。

| プロトコル | 役割・適用範囲 |
| :--- | :--- |
| [PLANNING](protocols/PLANNING_PROTOCOL.md) | **管理**: セッション管理、SSoT 役割定義、タスク管理。 |
| [HDD](protocols/HDD_PROTOCOL.md) | **Build**: 仕様先行・検証主導の開発サイクル。 |
| [FIX](protocols/FIX_PROTOCOL.md) | **Fix**: 診断先行・最小変更の修正サイクル。 |
| [RR-15](protocols/RELIABLE_RUST_15.md) | **品質**: Rust 実装における絶対的安全制約 15 項目。 |
| [NAMING](protocols/NAMING_CONVENTION.md) | **命名**: RFC 0430 に準拠した一貫した命名規則。 |
| [UI_DESIGN](protocols/UI_DESIGN_PROTOCOL.md) | **UI**: UI コンポーネントの設計、整合性、UX 基準。 |
| [COMPLIANCE](protocols/COMPLIANCE_STRATEGY.md) | **監査**: ISO 32000 準拠性および品質ゲートの定義。 |
| [PORTABILITY](protocols/PLANNING_PROTOCOL.md#5-ポータビリティと匿名性の確保-portability--privacy) | **共通**: ポータビリティと匿名性の確保。 |

---

## 4. プロジェクト情報

- **Target**: ISO 32000-2:2020 (PDF 2.0) Only
- **MSRV**: 1.85.0 / Edition 2024
