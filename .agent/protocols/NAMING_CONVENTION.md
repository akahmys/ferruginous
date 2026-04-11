# Naming Convention Protocol (RFC 0430 Alignment)

このドキュメントは、[Rust RFC 0430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md) に準拠した Ferruginous プロジェクトの公式命名規則を定義する。

> [!IMPORTANT]
> **優先順位**: PDF 仕様書（ISO 32000-2）の用語と Rust の慣習が衝突する場合、原則として **Rust の共通慣習（Naming Context）を優先**する。

---

## 1. ケース・スタイル (Casing Strategy)

- **規約**: 型・トレイトは `UpperCamelCase`、関数・変数は `snake_case` を徹底せよ。
- **目的**: Rust コンパイルの慣習（RFC 0430）に合わせることで、警告をゼロにし可読性を最大化する。
- **判定基準**: Clippy による命名警告がゼロであること。

## 2. Getter と変換 (Getters & Conversions)

- **規約**: 直接取得に `get_` を使用せず、変換には `as_`（参照）、`to_`（生成）、`into_`（消費）を使い分けよ。
- **目的**: Rust 特有の所有権セマンティクスをメソッド名から直感的に理解可能にする。
- **判定基準**: SDK 公開 API のシグネチャにおいて `get_` プレフィックスが排除されていること。

## 3. PDF 用語の調和 (PDF Domain Mapping)

- **規約**: PDF 仕様の用語（MediaBox 等）は保持するが、ケースは Rust 規約（media_box）に従え。略語も通常の単語（PdfError）として扱う。
- **目的**: ドメイン固有の知識と言語の慣習を矛盾なく融合させる。
- **判定基準**: Arlington Model のキー名と Rust メソッド名の対応が機械的に変換可能であること。

---

## 4. 命名対応表

| 項目 | スタイル | 例 |
| :--- | :--- | :--- |
| **Types, Traits** | `UpperCamelCase` | `PdfResolver`, `Object` |
| **Enum Variants** | `UpperCamelCase` | `Dictionary`, `String` |
| **Functions, Methods** | `snake_case` | `resolve()`, `media_box()` |
| **Variables, Fields** | `snake_case` | `local_name`, `page_dict` |
| **Constants** | `SCREAMING_SNAKE` | `MAX_STREAM_SIZE` |
