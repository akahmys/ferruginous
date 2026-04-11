# ISO 32000 準拠検証戦略 (Compliance Strategy)

> [!IMPORTANT]
> 「規格への適合」は Ferruginous の生命線である。Arlington Model, VeraPDF, 差分テストを統合し、完全準拠を機械的に証明せよ。

## 1. 検証レイヤー (Verification Layers)

- **規約**: Arlington Model による文法検証、VeraPDF による構造検証、および他エンジンとの比較レンダリングを統合せよ。
- **目的**: PDF 規格の膨大な仕様に対して、多角的な自動検証による裏付けを行う。
- **判定基準**: `scripts/safety_vitals.sh` および `cargo test` が全項目パスしていること。

## 2. 品質ゲート (Zero-Warning Gate)

- **規約**: `clippy::pedantic` 警告ゼロを義務化し、MSRV 1.85.0 への互換性を維持せよ。
- **目的**: 潜在的な不具合や技術的負債をソースレベルで根絶し、長期的な保守性能を確保する。
- **判定基準**: CI またはローカルでの `cargo clippy` 実行時に警告が 1 件も出力されないこと。

## 3. 証跡と Clause 紐付け (Evidence & Clauses)

- **規約**: 実装およびテストに ISO 32000-2:2020 の該当 Clause 番号を明記し、`walkthrough.md` にエビデンスを記録せよ。
- **目的**: どのコードが規格のどの部分を実装しているかを明文化し、監査を容易にする。
- **判定基準**: 全ての公開 API または重要なロジックに Clause 番号のコメントが含まれていること。

---

## 4. 監査ステップ

1. **実装前**: `specs/` と規格書の Clause を照合。
2. **実装中**: 型安全性による不正データ排除（RR-15 遵守）。
3. **報告前**: `verify_compliance.sh` の実行とパス。
4. **同期**: `sync_docs` による整合性証明。
