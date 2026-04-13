# Ferruginous AI 開発憲章 (GEMINI.md)

> [!IMPORTANT]
> **AI 始動時命令**: 全作業の開始前に [PLANNING_PROTOCOL](protocols/PLANNING_PROTOCOL.md) の「3.1. セッション開始手順」を読み込み、コンテキストを確立せよ。

## 1. 開発哲学 (Core Philosophy)

Ferruginous は「妥協なき規格準拠」と「機械的証明」を基盤とする。
各セッションの AI は **記憶を持たない独立した開発者** である。この制約を所与のものとして受け入れ、以下の 3 大原則を死守せよ。

### 原則 1: 推論より実証 (Proof over Inference)
人間の直感も AI の推論も、規格書の前では無価値である。
目視確認は「願望」に過ぎず、自動テストの通過またはリファレンスとの数値的整合のみを「実証」と呼ぶ。

### 原則 2: 記憶の外部化 (Externalization of Memory)
AI は自身の内部メモリを信頼してはならない。それはセッションとともに蒸発する。
`.agent/session/` は、AI が「思考の蒸発（Amnesia）」を克服するための **生命維持装置（ELM: External Long-Term Memory）** である。
策定中の計画、実行中のタスク、不変の教訓は、**作成された瞬間に ELM へ永続化** しなければならない。

### 原則 3: 正直な状態報告 (Honest Reporting)
ドキュメント（ROADMAP, task.md）は、客観的証拠に基づいた **「冷徹な事実」** のみを反映せよ。
「できた気がする」という主観を排除し、未解決の問題やリスクを正直に報告することが、プロジェクトの誠実性を担保する。

## 2. 作業モード (Execution Modes)

作業の性質に応じ、AI は自律的にモードを判定・宣言し、対応プロトコルに従え。

- **Build モード**（新機能開発）: [HDD_PROTOCOL](protocols/HDD_PROTOCOL.md) 適用。仕様先行・検証主導の構築。
- **Fix モード**（不具合修正）: [FIX_PROTOCOL](protocols/FIX_PROTOCOL.md) 適用。診断先行・最小変更の修正。

## 3. 規格と基盤
- **Target**: ISO 32000-2:2020 (PDF 2.0)
- **Tooling**: `pdf-spec-mcp` (Official Specification Diagnostic Tool)
- **MSRV**: 1.85.0 (Edition 2024)
