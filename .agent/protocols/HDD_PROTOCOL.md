# Harness-Driven Development (HDD) Protocol

> [!IMPORTANT]
> 本プロトコルは **Build モード**（新機能の開発）に適用する。不具合の修正には [FIX_PROTOCOL](FIX_PROTOCOL.md) を使用せよ。
> 「推測」を排し「機械的証明」を最優先とする。実装に先行して検証環境を構築し、外部仕様（ISO 32000-2）との整合性を静的・動的に証明せよ。

## 1. スペック・ソース (Spec-Source)

- **規約**: 実装前に必ず `specs/` または ISO 32000-2 の該当 Clause を特定し、根拠を明示せよ。
- **目的**: 開発者の主観を排し、国際規格への完全準拠を担保する。
- **判定基準**: `implementation_plan.md` またはコードコメントに Clause 番号が記載されていること。

## 2. フェイル・ファスト (Fail-Fast)

- **規約**: ロジック実装前に、必ず「期待通りに失敗するテスト（Harness）」を記述・実行せよ。
- **目的**: テスト自体が正しく挙動することを確認し、バグの早期発見を促す。
- **判定基準**: `.agent/session/task.md` において実装工程より前にハーネス構築が完了していること。

## 3. クロス・バリデーション (Cross-Validation)

- **規約**: SDK ロジックは単体テストで、UI 挙動は統合テストまたは視覚的スナップショットで実証せよ。
- **目的**: 複数の検証レイヤーを設けることで、論理と表示の両面から品質を担保する。
- **判定基準**: `.agent/session/walkthrough.md` にテスト結果のエビデンスが添付されていること。

---

## 4. 開発プロセス

1. **Plan**: `implementation_plan.md` で検証方法を合意。
2. **Harness**: 失敗するテストを構築。
3. **Execute**: [RR-15](RELIABLE_RUST_15.md) を遵守しつつ実装。
4. **Verify**: `scripts/verify_compliance.sh` および境界値テストを完遂。
5. **Sync**: [sync_docs](../workflows/sync_docs.md) で文書を最新化。

---

## 5. UI 統合時の特殊要件

- **Visual Regression**: UI 変更時は、期待される描画結果のスクリーンショット又は記述的なプロパティ検証を含めること。
- **Prop-to-Sdk**: UI 上の操作が SDK 状態に正しく正規化されて伝搬されるかを、状態追跡テストで検証せよ。

---

## 6. 完了ゲート (Completion Gate)

マイルストーンを「完了」と宣言するには、以下の **すべて** を満たすこと。

- **テストの存在 [MUST]**: 対象機能のテストが test suite に存在し、`cargo test` で PASS すること。
- **リグレッションなし [MUST]**: 全テストが PASS し、`.agent/session/regression_log.md` にリグレッションなしの記録があること。
- **エビデンス [MUST]**: テスト出力が `.agent/session/walkthrough.md` に添付されていること。
- **禁止事項**: 自動テスト無しに「完了」と宣言してはならない。ROADMAP の [完了] マーカーはユーザー承認後のみ付与。
