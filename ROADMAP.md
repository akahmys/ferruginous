# Ferruginous Roadmap (ISO 32000-2:2020 Strict)

## 1. 開発憲章 (Constitution)

- **[RR-15](.agent/protocols/RELIABLE_RUST_15.md)**: 15 の安全性制約を機械的に適用。
- **[HDD](.agent/protocols/HDD_PROTOCOL.md)**: 仕様先行・自動検証ハーネス駆動を徹底。
- **Target**: ISO 32000-2:2020 (PDF 2.0) Only
- **MSRV**: 1.94 / Edition 2024

## 2. 開発原則 (Parallel Principles)

- **Track-Driven**: SDK (Track S) と App (Track A) を並行トラックとして定義し、連動させつつ開発を加速する。
- **SSoT Sync**: 各トラック babysits の成果物は必ず `specs/` 配下の仕様書および `ROADMAP.md` と同期させる。

## 3. 開発フェーズ (AI Implementation Phases)

### Phase 1: Core Foundation (基礎構造) [完了]

- **M1-M4: 物理構造の確立**: COS解析、DrawOp、DisplayList。
- **M5-M8: 論理構造とリゾルバ**: ページツリー、リソース辞書、間接オブジェクト解決。

### Phase 2: Content Parsing (コンテンツ解析) [完了]

- **M9-M14: フィルタとグラフィックス命令**: FlateDecode, ASCIIHexDecode, パス描画。

### Phase 3: SDK Refinement (SDK の洗練) [完了]

- **M15-M18: リソース管理と整合性検証**: 画像、フォント基盤、Arlington 検証。

### Phase 4: Display & UX (表示と体験) [完了]

- **M23-26: Vello レンダリング統合**: WGPU/Vello パイプラインの Eframe 組み込み。

### Phase 5: Interactive Foundations (インタラクティブ基盤) [完了]

- **M30-A: Unicode 抽出とコピー (Copy & Unicode)** [完了]
  - 選択エリアの Unicode 復元とクリップボード連携 (Clause 9.10)。
- **M30-S: インクリメンタル保存の実態化 (Save Implementation)** [完了]
  - 物理シリアライザの統合による変更箇所のファイル永続化 (Clause 7.5.6)。
- **M31-A: 注釈編集 UI (Annotation UI)** [完了]
  - ハイライト、矩形、矢印、フリーハンドのビジュアル編集 (Clause 12.5)。
- **M32-A: 全文検索とアウトライン (Search & Outline)** [完了]
  - 文書内検索とサイドバー目次表示 (Clause 12.3.3)。

### Phase 6: Forms & Layers (フォームとレイヤー) [完了]

- **M34-S: AcroForms (インタラクティブフォーム)**
  - フォームフィールドの解析と外観ストリーム生成 (Clause 12.7)。
- **M35-A: フォーム入力・レイヤー操作 (UI Integration)**
  - UI 上での入力保存と OCG (レイヤー) 切り替え (Clause 14.11)。

### Phase 7: Advanced Graphics & Compliance (高度な描画と色彩) [完了]

- **M36-S: ICC プロファイルとカラー管理** [完了]
  - カラーマネジメントシステム (lcms2 等) の統合 (Clause 8.6)。
- **M37-S: 透明グループと高度なシェーディング** [完了]
  - 透過グループ、ブレンドモード、Type 4-7 シェーディング (Clause 8.7, 11)。

### Phase 8: Security & Trust (セキュリティと信頼性) [完了]

- **M38-S: AES-256 (Rev 6) 暗号化** [完了]
  - 規格準拠の高度なパスワード保護・復号 (Clause 7.6.4)。
- **M39-S: デジタル署名 (PAdES/LTV)** [完了]
  - 公開鍵基盤による署名の検証と付与 (Clause 12.8)。
- **M40-A: セキュアな墨消し (Redaction)** [完了]
  - コンテンツの物理的・不可逆的な削除 (Clause 12.5.6.23)。

### Phase 9: Accessibility & Structure (アクセシビリティ) [完了]

- **M41-S: 論理構造と Tagged PDF** [完了]
  - RoleMap/ClassMap 解決と文書構造ツリーの解析 (Clause 14.7)。
- **M42-A: 構造表示とタグ検証 UI** [完了]
  - 構造ツリーのサイドバー表示と Tagged PDF 整合性検証 (Clause 14.8)。

### Phase 10: Finalization & Release (最終統合) [完了]

- **M43-S: 3D & マルチメディア解析** [完了]
  - RichMedia および 3D アノテーションのパースと仕様確認 (Clause 13)。
- **M44-S: Arlington 完全適合試験** [完了]
  - 全オブジェクトのスキーマ自動再帰検証による品質保証。
- **M45: 最適化と最終統合** [完了]
  - Rayon による並列レンダリング最適化と UI 統合の完了。

### Phase 11: Governance & Quality (ガバナンスと品質向上) [完了]

- **M46-G: 統治プロトコルの再編**: `GEMINI.md`（憲章）と `PLANNING_PROTOCOL.md`（管理）の役割分離と重複排除。 [完了]
- **M47-Q: RR-15 コンプライアンスの強化**: `verify_compliance.sh` の整備と SDK/UI 全域での警告・違反ゼロ達成。 [完了]
- **M48-A: UI レイヤーの近代化**: ガード節によるロジックの平坦化、および `unwrap()` 排除による不変条件の強化。 [完了]

### Phase 12: Professional Orchestration (プロフェッショナル統合) [完了]

- **M49-A: ページ・オーケストレーター (UI)**: サムネイルグリッドによる視覚的なページ並べ替え・抽出・結合 UI。 [完了]
- **M49-S: 文書操作 API (SDK)**: リソースの不整合を発生させない安全なページツリー操作と、他文書からのページインポート。 [完了]

### Phase 13: Engineering Precision (エンジニアリング精度) [完了]

- **M50-S: 幾何学的近傍検索 (SDK)**: ベクトルパスへの精密なスナップポイント計算エンジン。 [完了]
- **M50-A: スナップ UX & 精密計測 (UI)**: CAD 級のスナップ機能による、正確な距離・面積のビジュアル計測とスケール管理。 [完了]

### Phase 14: Semantic Intelligence (セマンティック・インテリジェンス) [完了]

- **M51-S: 自動タグ構造化 & パターン検出 (SDK)**: AI による論理構造 (Tagged PDF) の自動生成と、機密情報のパターン抽出。 [完了]
- **M51-A: タグ・ツリー・エディタ (UI)**: 論理構造をマウス操作で「修復」するビジュアルエディタと、墨消しアシスタント。 [完了]

### Phase 15: Creative Mastery (クリエイティブ・マスター) [完了]

- **M52-A: モードレス_ワークフロー (UI)**: 全ての固定メニューを排した、選択オブジェクト駆動型のコンテキストメニュー UX。 [完了]
- **M52-S: リフロー編集 & 3D レンダリング (SDK)**: 段落単位でのテキストリフロー編集、および WGPU による U3D/PRC モデルの実レンダリング。 [完了]

### Phase 16: Architectural Resilience [完了]

- **M53-S: 描画バックエンドの抽象化 (Render Bridge)** [x]
  - Vello への直接依存を排除し、`RenderBackend` トレイトによる抽象化レイヤーを構築。
- **M54-G: プラットフォーム非依存性の確立** [x]
  - WASM / Web Canvas への将来的なブリッジ接続を考慮したインターフェースの定義と、`ferruginous-ui` の依存構造整理。

### Phase 16.5: Quality & Feature Refinement [完了]

- **M55-Q: 技術的負債の解消 (Sprint)** [x]
  - 全 SDK/UI レイヤーでの `missing_docs`, `redundant_clone`, 安全な型キャストの徹底。
- **M56-G: ドキュメント品質の向上** [x]
  - 主要 API への ISO 32000-2 条項引用に基づく解説追加。

### Phase 18: Multibyte Text Rendering Precision (マルチバイトテキスト描画の精密化) [完了]

> [!NOTE]
> **全工程完了 (2026-04-14)**:
> CIDToGIDMap の解決、垂直原点シフト、および WMode に基づくグリフ回転の実装と検証が完了。
> `diag_layout` および `verify_compliance.sh` による最終品質保証をパス。

- **M59-S: FontMatrix 標準化とコンポジットフォント同期 (SDK)** [x]
  - ISO 32000-2 準拠のデフォルト FontMatrix (0.001) の統一と、Type 0 フォントにおける子要素 CIDFont からの行列継承の実装。
- **M60-S: グリフ幅の厳密同期 (SDK)** [x]
  - PDF の `W` (Widths) 配列とフォントファイルのグリフメトリクスを強制同期させ、日本語文字の重なり（バンチング）を物理的に解消。
- **M61-A: レンダラー座標変換の完全同期 (UI)** [x]
  - 累積的な座標変換行列 (CTM) をテキスト描画に完全に統合し、図形や枠線との位置ずれを 100% 解消。

## 4. 運用ルール (Process Rules)

- **Parallel Sync**: `task.md` で Track S と Track A の同期ポイントを管理せよ。
- **Atomic Sync**: 各報告前に `task.md` および `ROADMAP.md` を同期。
## 5. 次世代ロードマップ案: Ferruginous "Sentinel" v2.0

現在までの Phase 18 の完遂を受け、エンジンの「堅牢性」と「ビジュアル・体験」を両立させる次世代フェーズを定義します。

### Phase 19: High-End Compliance & Archiving (規格適合の頂点へ) [ ]
プロフェッショナル向け規格の完全制覇。
- **M62-S: PDF/A-4 & PDF/X-6 完全準拠 (Strict Write)**
  - 長期保存および商用印刷の最新規格への適合。
- **M63-S: 高精度カラーマネジメント (Spot Color & ICC v4)**
  - 特色や分光定義色の高度な再現。

### Phase 20: Visual Excellence & Design System (UI ブラッシュアップ) [ ]
Egui をベースとした独自デザインシステム "Sentinel UI" の構築。
- **M64-A: プレミアム UI テーマの構築**
  - グラスモフィズム、HSL 色彩設計、モダンなタイポグラフィの導入。
- **M65-A: モーションとマイクロインタラクション**
  - 滑らかなパネル遷移と直感的なホバーフィードバック。

### Phase 21: Performance & Universal Platform (汎用性と Web) [ ]
- **M66-S: 線形化 (Linearization) サポート**
  - 巨大ファイルの即時表示（Fast Web View）。
- **M67-P: WGPU-WASM パフォーマンス・スプリント**
  - ブラウザ上でのレンダリング速度をデスクトップ級に引き上げ。
- **M68-A: リフロー閲覧モード (Liquid Mode) の初期実装**

### Phase 22: Advanced Interactive Core (高度な相互作用) [ ]
- **M69-S: Safe AcroJS Sandbox (RR-15 準拠)**
  - 規格準拠の JS 実行環境をセキュアなサンドボックスで実装。
- **M70-A: インタラクティブ・フォーム UI の刷新**
  - リアルタイムバリデーションとシームレスな入力体験。

### Phase 23: Data PDF & Hybrid Containers (データコンテナ化) [ ]
- **M71-S: 関連ファイル (Associated Files) の双方向管理**
  - PDF をデータコンテナとして活用する Clause 14.13 の実装。

### Phase 24: AI-Native Document Intelligence (AI 連携 - 延期) [ ]
- **M72-S: セマンティック抽出と LLM 連携**
- **M73-A: 対話型 AI アシスタント UI**
