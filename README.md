# Ferruginous: 人間と AI の共創による PDF 2.0 ツールキット

Ferruginous（フェルジナス）は、ISO 32000-2 (PDF 2.0) 準拠を目指して開発中の、オープンソースの PDF ツールキットです。
本プロジェクトは、AI エージェント **Antigravity** を活用したプログラミング課題として、PDF 規格に挑む実験的な個人プロジェクトです。

## プロジェクトの概要

このプロジェクトの最大の特徴は、人間と AI の共同開発体制にあります。
人間（開発者）は基本方針の決定とマイルストーンのレビューに専念し、**具体的な実装の大部分は Antigravity が自律的に担当しています。** また、自らの動作を律する安全性プロトコルやガバナンスルール自体も Antigravity 自身が実装・改定していくという、自己洗練型の開発プロセスを採用しています。

Rust の安全性を最大限に引き出した「壊れにくい」実装と、GPU 描画エンジン [Vello](https://github.com/linebender/vello) による軽快な動作を両立した、モダンな PDF 環境の構築を目標としています。

現在、プロジェクトは **全 Phase 15** (プロフェッショナル統合とクリエイティブ・マスター) を完了し、次世代の PDF 編集基盤が確立した段階です。

### ✅ 実装済みの全機能

- **解析とレンダリング (Phase 1-4)**: PDF 文書構造を解析し、Vello による GPU 描画をサポート。
- **検索とUnicode抽出 (Phase 5)**: Unicode 復元によるテキスト抽出と全文検索。
- **注釈と編集 (Phase 5)**: ハイライト、矩形注釈の付与とインクリメンタル保存。
- **フォームとレイヤー (Phase 6)**: AcroForm の入力・保存と OCG レイヤー制御。
- **高度な描画 (Phase 7)**: ICC カラー管理、透明グループ、複雑なシェーディング。
- **セキュリティと信頼性 (Phase 8)**: AES-256 Rev 6 暗号化、PAdES 電子署名(LTV)、墨消し。
- **論理構造とアクセシビリティ (Phase 9)**: Tagged PDF 解析、RoleMap/ClassMap 解決、整合性検証 UI。
- **マルチメディアと品質保証 (Phase 10)**: RichMedia/3D 解析、Arlington モデルによる再帰的検証、Rayon 並列最適化。
- **プロフェッショナル統合 (Phase 12-13)**: サムネイル駆動のページ操作 (Orchestrator)、CAD 級の精密スナップと計測ツール。
- **セマンティック & クリエイティブ (Phase 14-15)**: AI によるタグ構造自動生成・修復、選択オブジェクト駆動型のモードレス・コンテキスト UX。

### 🗓 今後の展望

- **Arlington JavaScript 述語**: 複雑な動的条件の完全パース。
- **3D 投影描画**: Vello を用いた 3D モデルのリアルタイムレンダリング対応。
- **アプリパッケージ**: 各 OS 向けのネイティブインストーラ自動ビルド。

詳細な実装状況は [ROADMAP.md](ROADMAP.md) に記録しています。

## 技術的な構成

- **言語**: Rust (Edition 2024 / MSRV 1.94)
- **描画**: [Vello](https://github.com/linebender/vello) / WGPU
- **UI フレームワーク**: [egui](https://github.com/emilk/egui) / eframe (0.33.1)
- **準拠規格**: ISO 32000-2:2020

## 開発環境

- **マシン**: MacBook Air (Intel Core i5 / 16GB Memory)
- **OS**: macOS 15.7.4
- **AI エージェント**: Antigravity (Gemini)
- **開発ツール**: Cargo, Clippy, Rustfmt

## 開発を支えるルール

個人のプロジェクトですが、Antigravity が自律的に遵守し、かつ自ら改善し続けている以下のルールによって品質を保っています。

1. **HDD (Harness-Driven Development)**: 機能を記述する前に検証用の「ハーネス」を用意し、正しく動くことを常に確認しています。
2. **RR-15 (Reliable Rust 15)**: 実行時に予測できないクラッシュを防ぐため、15 項目の厳格な安全性制約を守っています。**現在は SDK および UI レイヤーの全てで 100% 準拠を達成しています。**
3. **Automated Governance**: AI (Antigravity) と協力して、仕様書と実装の整合性を自動で保つ仕組み（SSoT 同期等）を取り入れています。

## ファイルの構成

- `crates/`: SDK 本体やレンダリング、UI 等のソースコード。
- `specs/`: 設計の「正典」として扱っている技術仕様書。
- `samples/`: テスト用の PDF サンプル。
- `scripts/`: 品質監査やコンプライアンス検証を行うためのスクリプト。

---

### 開発者・AI 向けの情報

詳細なプロトコルやワークフローについては、[.agent/](.agent/) ディレクトリに集約されています。

- [AI 開発憲章 (GEMINI.md)](.agent/GEMINI.md): AI の行動原則。
- [管理プロトコル (PLANNING_PROTOCOL.md)](.agent/protocols/PLANNING_PROTOCOL.md): 計画・同期・管理ルール。
- [品質プロトコル (RELIABLE_RUST_15.md)](.agent/protocols/RELIABLE_RUST_15.md): 安全制約 15 条。


---
© 2026 Ferruginous Project.
