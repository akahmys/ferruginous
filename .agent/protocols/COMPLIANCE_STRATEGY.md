# ISO 32000 準拠検証戦略 (Compliance Strategy)

> [!IMPORTANT]
> **規格への適合は Ferruginous の存在意義である。**
> 主観を排し、MCP ツール、Arlington Model、および差分テストを用いて、完全準拠を客観的に証明せよ。

## 1. 静的検証（規格照合）
- **MCP 監査**: `pdf-spec-mcp` を用い、実装しようとしているロジックが規格書のどの条項（Clause）に紐づくかを明文化する。
- **Arlington 文法**: `ferruginous-sdk` の辞書・ストリーム生成が Arlington PDF Model と矛盾しないことを静的に検証する。

## 2. 動的検証（レンダリング・構造）
- **VeraPDF 監査**: 出力された PDF が VeraPDF 等のバリデータで「PDF 2.0 準拠」と判定されることを目指す。
- **差分テスト**: 他のリファレンス実装（pdfium 等）と描画結果を比較し、ピクセル単位または行列単位での整合性を確認する。

## 3. 品質ゲート (Zero-Warning Gate)
- **Clippy**: `clippy::all`, `clippy::pedantic` において警告ゼロ。
- **Coverage**: 重要な変換ロジック、行列計算、色空間処理におけるテスト網羅率の維持。

## 4. 証跡の永続化
- すべての検証結果は、単なる「通過報告」ではなく、**具体的な出力ログや Clause 番号と共に `walkthrough.md` に ELM 永続化（即時同期）** されなければならない。
- `specs/` 配下のドキュメントは、常に最新の規格解釈を反映するよう `sync_docs` で同期せよ。
