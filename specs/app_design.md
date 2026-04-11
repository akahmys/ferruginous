# Technical Specification: Ferruginous UI

> [!IMPORTANT]
> `ferruginous-sdk` の解析能力と Vello レンダリングを統合し、Adobe Acrobat Pro を凌駕する操作性と機能性を実現する PDF 閲覧・物理編集ソリューション。

## 1. 設計思想

本プロジェクトは **[RR-15](../.agent/protocols/RELIABLE_RUST_15.md)** および **[HDD](../.agent/protocols/HDD_PROTOCOL.md)** をアプリ層へ拡張して適用する。描画には `vello` によるハードウェア加速と、Engine が `DisplayList` を生成し Renderer が非同期消費する Pull-type Architecture を採用。

### 1.1 究極のアプリに向けた 4 本柱 (The 4 Pillars)

1. **Zero-Latency Rendering**: GPU 演算 (Vello) による解像度非依存・遅延ゼロの描画体験。
2. **Engineering-Grade Precision**: ベクトルデータに基づく CAD 級のスナップと実寸計測。
3. **Semantic Editor**: オブジェクト指向の UI による、直感的な構造編集とタグ修正。
4. **Transparent Governance**: 規格準拠性とセキュリティの完全な視覚化と保証。

## 2. システムアーキテクチャ

### 2.1 描画・解析フロー

1. **Parse**: `loader` による PDF 2.0 読み込みと `Catalog` 構築。
2. **Validate**: `arlington` による辞書妥当性検証。
3. **Interpret**: `engine` によるコンテンツストリーム走査。
4. **Transform**: PDF ユーザ空間から `kurbo::Affine` でデバイス座標へ変換。
5. **State Update**: `GraphicsState` スタック更新。
6. **Issue**: `DrawOp` を `DisplayList` へ蓄積。
7. **Render**: `vello` 側で命令を GPU 描画。

### 2.2 技術スタック

- **言語・基盤**: Rust 1.94 / Edition 2024
- **レンダリングエンジン**: `Vello` (次世代ベクトルグラフィックス・レンダラ)
- **グラフィックス API**: `WGPU` 28.0 (WebGPU 準拠)
- **UI フレームワーク**: `egui`/`eframe` 0.33.1
- **ウィンドウ管理**: `winit` 0.30

## 3. UI/UX 戦略: "Contextual Magic"

- **Menu-less Workflow**: 巨大な固定メニューを廃止し、オブジェクト選択時に最適な操作オプションを提示するコンテキスト指向 UX。
- **Infinite Workspace**: 複数 PDF を一つのキャンバスで管理し、ページや要素を自由にオーケストレーション可能にする。

## 4. 座標変換モデル

$$P_{screen} = M_{zoom} \cdot M_{pan} \cdot M_{pdf\_to\_screen} \cdot P_{user}$$

- **$M_{pdf\_to\_screen}$**: MediaBox 基軸の Y 軸反転・原点オフセット。
- **$M_{zoom}$ / $M_{pan}$**: ユーザー操作（拡縮・移動）変換。

## 5. UI 構成 (ferruginous-ui)

- **Canvas**: `vello` を用いて、PDF ベクトルデータと動的オーバーレイ（計測線、スナップ点など）を統合描画。
- **Control**: `egui` によるモードレスなフローティングメニュー。
- **Inspector**: Arlington 検証結果と Tag 構造の視覚化エディタ。
- **Editor Bridge**: UI での破壊的操作を SDK の `PdfEditor` へ伝搬し、インクリメンタル保存。

## 6. 品質ガードレール

- **Audit**: `verify_compliance.sh` による安全性監査。
- **Fail-safe**: 規格違反時もセーフモードで描画を継続し、ユーザーによる構造修復を支援する。
