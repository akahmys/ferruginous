# Technical Specification: Ferruginous SDK

> [!IMPORTANT]
> ISO 32000-2:2020 Strict 準拠。Pure Rust 製 PDF 閲覧・編集エンジン。RR-15 及 HDD プロトコルを「開発憲章」とする。

## 1. 開発プロトコル

本プロジェクトは **[RR-15](../.agent/protocols/RELIABLE_RUST_15.md)** (安全性) および **[HDD](../.agent/protocols/HDD_PROTOCOL.md)** (プロセス) を「開発憲章」として遵守する。

## 2. 4 レイヤー構造

### A. Object Layer (Physical & Logical)

- **`lexer.rs`/`loader.rs`**: トークナイズとストリーム読み込み。
- **`xref.rs`/`trailer.rs`**: ISO 32000-2 推奨 XRef ストリーム/ハイブリッド参照処理。
- **`resolver.rs`**: 間接参照解決。Arlington 検証エントリ点。
- **`security.rs`**: AES-256 Revision 6 (PDF 2.0) 復号・暗号化。

### B. Logical Layer (Structure)

本プロジェクトは **[RR-15](../.agent/protocols/RELIABLE_RUST_15.md)** および **[HDD](../.agent/protocols/HDD_PROTOCOL.md)** をアプリ層へ拡張して適用する。描画には `vello` によるハードウェア加速と、Engine が `DisplayList` を生成し Renderer が非同期消費する Pull-type Architecture を採用。

### C. Engine Layer (The Interpreter)

- **`content.rs`**: コンテンツストリームパース命令ループ。
- **`graphics.rs`**: ISO 32000-2 Table 51 に基づく Graphics State Stack。
- **`text.rs`/`font.rs`/`cmap.rs`**: UTF-8 グリフ配置と CMap 解決。
  - **Precision Rendering**: ISO 32000-2 Table 112 に基づく FontMatrix (0.001) の標準化と、`W` 配列によるグリフ幅の厳密な同期（Width Synchronization）をサポート。

### D. Export Layer (The Writer)

- **`editor.rs`**: 非破壊編集。
- **`writer.rs`**: 物理シリアライザ。

## 3. 技術スタック (Technology Stack)

SDK の決定論的かつ安全な動作を保証するため、以下のライブラリを選定している。

- **言語・基盤**: Rust 1.94 / Edition 2024 (SSoT: Cargo.toml)
- **パーサ**: `nom` 7.1 (Parser combinators)
  - 高速かつ安全なバイナリ/テキスト解析。
- **圧縮/展開**: `miniz_oxide` 0.7 (Pure-rust Flate/Zlib)
  - `FlateDecode` の解凍に使用。
- **暗号化**: `aes` 0.8 / `cbc` 0.1 / `md-5` 0.10 / `sha2` 0.10
  - PDF 標準セキュリティハンドラの暗号ロジック。
- **幾何計算**: `kurbo` 0.13 (2D geometry)
  - 描画命令の座標変換およびベジェ曲線演算。
- **画像フィルタ**: `jpeg-decoder` 0.3.2 / `hayro-jpeg2000` 0.3.4
  - `DCTDecode` (JPEG) および `JPXDecode` (JPEG 2000) の展開。

## 4. 物理構造の解釈

### GraphicsState (PDF 2.0 準拠)

```rust
pub struct GraphicsState {
    pub ctm: kurbo::Affine,
    pub clipping_path: kurbo::BezPath,
    pub blend_mode: BlendMode,          // PDF 2.0
    pub alpha_constant: f32,
    pub black_point_compensation: bool, // PDF 2.0 必須
    // ...
}
```

### DrawOp (Intermediate Representation)

```rust
pub enum DrawOp {
    PushState, PopState,
    FillPath { path: kurbo::BezPath, color: Color, opacity: f32 },
    StrokePath { path: kurbo::BezPath, color: Color, style: StrokeStyle },
    Text { glyphs: Vec<GlyphInstance>, font_id: ResourceId, size: f32 },
    Image { id: ResourceId, rect: kurbo::Rect },
    Clip(kurbo::BezPath, ClippingRule),
}
```

## 5. 物理シリアライザ (Physical Serializer) [M28-S]

### 設計方針

- **ISO 32000-2 準拠**: XRef Stream (PDF 2.0 推奨) 形式での出力を最優先とする。
- **非破壊編集 (Incremental Update)**: 既存バイナリの後方に「変更差分オブジェクト + 新規 XRef + 新規 Trailer」を追記する方式を採用。
- **RR-15 準拠 (決定論性)**: バイナリ出力時のハッシュ値整合性と、未定義オブジェクト参照の静的/動的排除。

### 主要コンポーネント

- **`writer.rs`**: 物理的な `std::io::Write` へのバイナリ書き込み抽象化。
- **`serialize/object.rs`**: 基本型（Dictionary, Array, String, Name）の再帰的出力ロジック。
- **`serialize/increment.rs`**: 既存 PDF ファイルへの追記と、`/Prev` リンクを持つ Trailer の生成。

## 6. ガードレール

- **静的解析**: `verify_compliance.sh` による RR-15 条項の強制適用。
- **動的検証**: Arlington 模型の述語評価エンジンによる、出力済み PDF の仕様準拠性（動的な条件を含む）のセルフ・オーディット。
- **エラー処理**: 書き込み失敗時、バッファの部分書き込みによるデータ破損を防止。

## 7. 堅牢性とセキュリティ方針 (Robustness & Security Policy)

Ferruginous PDF Engine は、実世界の「不完全な PDF」に対する高い耐性と、信頼性の高い「規格準拠 PDF」の出力を両立させるため、以下の基本方針を遵守する。

> [!TIP]
> **Liberal Read, Strict Write (堅牢性の原則)**
>
> - **読み込みは柔軟に**: 軽微な規格違反（キーワード前の不要な空白、非標準的な改行コード、ファイル先頭のゴミ等）は、構造が修復可能でセキュリティリスクがない限り、寛容に受け入れ る。他の主要な PDF ビューアで開けるファイルは可能な限り読み込めることを目標とする。
> - **書き出しは厳密に**: SDK が生成・編集する全てのバイト列は ISO 32000-2:2020 に 100% 準拠させる。不完全な構造を出力することは許容されない。

### セキュリティ・ガードレール (Security Guardrails)

柔軟な読み込みが脆弱性の温床にならないよう、以下の制限を強制する。

- **リソース制限**: `MAX_WS_ITER` (空白走査制限) や `MAX_STR_LEN` (文字列長制限)、デコンプレス後のメモリキャップなどの定数値を設け、DOS 攻撃を防止する。
- **階層深度の制限**: 再帰的なオブジェクト探索（PDF Pages Tree 等）においては、スタックオーバーフローを防ぐため、明示的な深度制限を設けた反復処理を行う。
- **決定論的動作**: 同一の入力に対しては常に同一の解析結果を生成し、環境依存の挙動を排除する。
