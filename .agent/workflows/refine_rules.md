---
description: 自己ルールの精緻化ワークフロー
---

# [Workflow] ルール精緻化 (Refine Rules)

予測可能性を高めるため、規約を自律改善（Self-Refinement）せよ。

## 手順

// turbo
1. **分析**: [analyze_friction](../skills/analyze_friction.md) スキルを実行し、摩擦の原因を特定.
2. **検証**: 改善案が [RR-15](../protocols/RELIABLE_RUST_15.md) の哲学に適合し、かつ現状の課題を解決できるか検証。
3. **運用の最適化**: 改善されたルールを、どのレイヤー（Protocol / Skill / Workflow）で運用するのが最も効率的か決定せよ。
4. **提案**: 改善案および運用の変更案をユーザーへ提示し、合意を形成せよ。
// turbo
4. **反映**: 承認後、規約ファイル（Constitution/Protocols）を修正し、[sync_docs](sync_docs.md) で反映を定着させよ。

> [!CAUTION]
> 基本原則（安全制約）を「開発のしやすさ」のために緩和することは厳禁。常に「より安全に、より機械的に」なる方向へ改善せよ。
