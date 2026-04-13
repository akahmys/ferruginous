# ISO 32000-2 準拠性に関する実装制限および簡略化の記録

本ドキュメントは、Ferruginous PDF エンジンの開発において、ISO 32000-2:2020 (PDF 2.0) 規格の完全な実装に対して、意図的に簡略化または制限を設けた項目を記録するものです。これらの決定は、Reliable Rust-15 (RR-15) コーディング規則の遵守、およびミッションクリティカルな環境での決定論的動作（Deterministic behavior）の確保を最優先とした結果です。

## 1. ストリーム・フィルタ (Stream Filters)

規格（Clause 7.4）では多様な圧縮アルゴリズムが定義されています。Ferruginous では、現在以下のフィルタに限定してサポートしています。

- **FlateDecode / ASCIIHexDecode**: 汎用的なデータ圧縮およびテキストベースの符号化として実装。
- **DCTDecode / JPXDecode**: JPEG および JPEG 2000 (hayro-jpeg2000 経由) 形式の画像デコードをサポート。
- **LZWDecode / CCITTFaxDecode / JBIG2Decode**: Phase 12 で実運用向けに完全実装。`weezl` (LZW), `fax` (CCITT G4), `justbig2` (JBIG2) を Safe Rust の制約内で統合し、スキャン文書やアーカイブ文書への対応を完了しました。
- **未実装項目**: `RunLengthDecode`。
  - **理由**: 利用頻度が極めて低く、他の優先度の高いフィルタ実装を優先しました。

## 2. グラフィックスと色空間 (Graphics & Color Spaces)

グラフィックス描画（Clause 8）については、主要なレンダリングモデルをカバーしていますが、以下の制限があります。

- **色空間 (Color Spaces)**: `DeviceGray`, `DeviceRGB`, `DeviceCMYK` の基本 3 種に加え、`ICCBased` を完全サポート (lcms2-rs 経由)。高精度なカラーマネジメントプロファイルに基づく色変換を実現しています。
- **パターンとシェーディング**: `Function-based Shading` (Type 1-3) および `Axial/Radial Shading` に加え、`Mesh Gradients` (Type 4-7) の完全な CPU テッセレーションをサポート。Coons Patch や Tensor-Product Patch の再帰的細分割 (Subdivision) により、規格準拠かつ高精細な描画が可能です。

## 3. フォントとテキスト (Fonts & Text)

テキスト処理（Clause 9）は、最も仕様が複雑な領域の一つです。

- **フォント形式**: `Type 1`, `TrueType`, `Type 0 (CIDFont)` をサポート。
- **進展 (Phase 11)**: 全ての TrueType/CIDFont に対して、抽出したグリフアウトライン（BezPath）を直接レンダリングするパイプラインを実装。WMode 1（垂直書き）のアドバンス計算、および CID メトリクス（/W, /DW）に完全準拠。
- **簡略化内容**: `Type 3`（ユーザー定義フォント）は、任意の描画命令を内包できるため、再帰呼び出しやスタックオーバーフローのリスク（RR-15 Rule 6 違反）を考慮して除外しています。

## 4. ドキュメント構造と走査 (Document Structure)

PDF 文書の論理・物理構造（Clause 7/14）は、再帰的な木構造で定義されます。

- **非再帰走査の強制**: 規格上は再帰的なツリーとして定義される `Page Tree` や `Resource Dictionary` ですが、RR-15 Rule 6（No Recursion）を遵守するため、全内部処理を明示的なスタック（`Vec`）を用いた反復処理に置き換えています。これにより、規格どおりの再帰的深さを持つファイルに対するスタックオーバーフローの懸念を完全に排除しました。

## 5. インタラクティブ機能 (Interactive Features)

フォーム、注釈、セキュリティ（Clause 12/13）に関しては、以下の設計指針を採っています。

- **静的ならびに動的解析**: `AcroForm` および `Digital Signatures` を完全統合。
  - **AcroForm**: フィールド値の一括取得 (export) および設定 (import) API を提供し、JSON 形式での外部連携が可能になりました。
  - **Digital Signatures**: 単なる構造検証を超え、`x509-parser` と `rsa` / `ed25519-dalek` を用いた暗号学的な署名検証（SHA-256 + 非対称鍵検証）を実装しました。
- **マルチメディア (Clause 13)**: `RichMedia` および `3D` アノテーションは、辞書構造の解析と Arlington モデルによる検証のみをサポートしています。
  - **制限**: U3D/PRC モデルの実レンダリングや、外部メディア（音声・ビデオ）のデコード・再生機能は、外部コーデックへの依存とランタイムの安全性確保の観点から、現時点では意図的に実装を見送っています。
- **制限事項**: `XFA` (XML Forms Architecture) は、その複雑さとセキュリティ上の攻撃ベクトルの多さを考慮し、非サポートとしています。

## 6. 自動検証とアクセシビリティ (Validation & Accessibility)

- **Arlington 述語 (Predicates)**: Phase 17 において、`nom` による AST パーサおよび再帰的評価エンジンを完全実装しました。これにより、ISO 32000-2 規格に含まれる複雑なバリデーション条件（SinceVersion, Required, Dependent key check 等）の動的評価が可能です。
- **Tagged PDF 修復**: `TaggedPdfValidator` は ISO 32000-2 Clause 14.8 への準拠性を検証しますが、欠落しているタグ構造の「自動推論・修復」機能は備えていません。

## 7. 方針: Liberal Read, Strict Write

Ferruginous PDF Engine は、読み込みにおいて最大限の互換性（Liberal Read）を確保しつつ、書き出しにおいて規格への完全準拠（Strict Write）を貫くことを基本方針としています。本ドキュメントに記載された制限事項の多くは、この方針に基づき、複雑でエラーを誘発しやすい古い仕様をあえて「書き出し」対象から除外することで、生成される PDF の品質と安全性を高めるための戦略的決定でもあります。

## 総評

Ferruginous は、PDF 2.0 の「全ての定義を読み取る」ことよりも、「読み取った定義が正しく、かつ安全に処理されること」を重視した実装となっています。将来的にこれらの制限を緩和する場合は、各マイルストーンで確立された RR-15 監査を再度パスさせる必要があります。
