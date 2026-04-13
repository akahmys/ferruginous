# Harness-Driven Development (HDD) Protocol

> [!IMPORTANT]
> **Build モード**: 新機能開発時に適用。
> 推論を排し「機械的証明」を最優先せよ。実装に先行して検証環境（Harness）を構築し、ISO 32000-2 との整合性を動的に証明せよ。

## 1. スペック・ファースト (Spec-First)
- **規約**: 実装前に必ず `pdf-spec-mcp` を使用して、該当する ISO 32000-2 の Clause（条項）および要求事項（shall/must）を抽出せよ。
- **目的**: 開発者の主観を排除し、国際規格への「完全準拠」を設計段階で担保する。
- **判定基準**: `implementation_plan.md` に Clause 番号と抽出された要件が引用されていること。

## 2. ハーネス先行 (Harness-First)
- **規約**: ロジック実装前に、必ず「期待通りに失敗するテスト」または「出力を検証するプローブ」を記述せよ。
- **目的**: テスト自体が要件を正しく反映していることを確認し、デバッグの迷走を防ぐ。
- **判定基準**: `task.md` において、ロジック実装より前にハーネスの構築が完了していること。

## 3. 回帰不能の証明 (Proof of Non-Regression)
- **SDK**: 単体テストおよび境界値テストで論理的正しさを証明。
- **UI**: 視覚的スナップショットまたは一意な ID を用いた状態遷移テストで表示の正しさを実証。

## 4. 実行プロセス
1.  **Define**: `pdf-spec-mcp` で法的要件を確認。
2.  **Plan**: `implementation_plan.md` を作成し、ELM へ即時永続化。
3.  **Harness**: 失敗するテストを構築。
4.  **Execute**: [RR-15](RELIABLE_RUST_15.md) を遵守して実装。
5.  **Verify**: `cargo test` および `verify_compliance.sh` を完遂。

## 5. 完了ゲート (Completion Gate)
- **テスト通過 [MUST]**: 新機能および既存機能の全テストが PASS すること。
- **エビデンス [MUST]**: テスト結果と Clause 紐付けが `walkthrough.md` に記録されていること。
- **ELM 同期 [MUST]**: すべての記録がプロジェクトディレクトリに永続化されていること。
