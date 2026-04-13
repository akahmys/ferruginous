# Session Handoff (2026-04-13) - ELM Lifecycle Ready

- **Mode**: Infrastructure (Setup & Refinement)
- **Status**: **完了 (基盤構築完了 - 解析フェーズへ)**
- **SSoT**: [PLANNING_PROTOCOL](../protocols/PLANNING_PROTOCOL.md)

## Current Context (脳内スナップショット)

本セッションで Ferruginous の開発基盤を「推論」から「規格実証」へと完全に移行した。

### 1. 外部基盤 (Tools)
- **pdf-spec-mcp**: `.agent/tools/pdf-spec-mcp` に構築済み。ISO 32000-2 規格への直接クエリが可能。
- **仕様書**: `specs/ISO_32000-2_sponsored-ec2.pdf` に正典を配置済み。

### 2. 統治基盤 (Governance)
- **ELM (External Long-Term Memory)**: `.agent/session/` が AI の生命維持装置として定義された。**「Artifact 作成と同時に ELM ミラーを生成する」** 規約が有効。
- **原則**: 「推論より実証」が憲章（GEMINI.md）として再定義された。

## Open Issues / Blockers

- **Phase 18 (日本語描画)**: 準備は整ったが、本質的な修正は未着手。
- **Compliance**: `verify_compliance.sh` により、`crates/ferruginous-sdk/examples/dump_page.rs` 等での `unwrap()` 違反が検出されている（優先度：低）。

## Next Action Entry Point

1.  **Fix モード** の開始。
2.  `pdf-spec-mcp` を使用した **Phase 18 の精密診断**。
    - 抽出対象: FontMatrix, CTM, Tfs (Text font size), W (Widths) の規格要件。
3.  ELM 規約の遵守（常に `session/` への即時永続化を伴うこと）。
