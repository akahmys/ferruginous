# Ferruginous AI 開発憲章 (GEMINI.md)

> [!IMPORTANT]
> **AI 始動時命令**: 全作業の開始前に、本ドキュメント及び `specs/` 配下の仕様書を読み込み、コンテキストの Single Source of Truth (SSoT) を確立せよ。

## 1. 開発哲学 (Core Philosophy)

Ferruginous は「妥協なき安全性」と「機械的証明」を基盤とする。AI は常に以下の原則を固守せよ。

1. **推論より実証**: 人間の推論は誤る。常に [HDD_PROTOCOL](protocols/HDD_PROTOCOL.md) に基づき、検証ハーネスによる実証を優先せよ。

2. **命令調の維持**: 思考プロセスを明示し、曖昧な「提案」ではなく、規約に基づく「決定」を行え。

3. **SSoT の遵守**: 記述が矛盾する場合、本ドキュメント（憲章）および [PLANNING_PROTOCOL](protocols/PLANNING_PROTOCOL.md)（実務）を最高位の判断基準とせよ。

---

## 2. 規約インデックス (Protocols Index)

実行する作業の性質に基づき、以下の各プロトコルを適宜参照・適用せよ。

| プロトコル | 役割・適用範囲 |
| :--- | :--- |
| [PLANNING](protocols/PLANNING_PROTOCOL.md) | **管理**: プロセスの進め方、SSoT 役割定義、タスク管理。 |
| [RR-15](protocols/RELIABLE_RUST_15.md) | **品質**: Rust 実装における絶対的安全制約 15 項目。 |
| [NAMING](protocols/NAMING_CONVENTION.md) | **命名**: RFC 0430 に準拠した一貫した命名規則。 |
| [HDD](protocols/HDD_PROTOCOL.md) | **開発**: 仕様第一・検証主導の開発サイクル（Harness-Driven）。 |
| [UI_DESIGN](protocols/UI_DESIGN_PROTOCOL.md) | **UI**: UI コンポーネントの設計、整合性、UX 基準。 |
| [COMPLIANCE](protocols/COMPLIANCE_STRATEGY.md) | **監査**: ISO 32000 準拠性および品質ゲートの定義。 |
| [PORTABILITY](protocols/PLANNING_PROTOCOL.md#5) | **共通**: ポータビリティと匿名性の確保（相対パス強制）。 |

---

