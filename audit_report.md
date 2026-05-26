# Ferruginous コードベース監査レポート

> [!NOTE]
> 本レポートは `2026-05-26` 時点のコードベース全体を対象としたスナップショット監査です。
> ビルド: ✅ 成功 / テスト: ✅ 全件パス (10件) / unsafe: ✅ 使用なし (`forbid` 設定済)

---

## 1. 重大度: Critical（即座に対応すべき問題）

### 1.1 デバッグ出力がプロダクションコードに残留
**影響範囲**: [writer.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/writer.rs)

`writer.rs` に **18箇所** の `eprintln!("DEBUG: ...")` が残留しています。これらは Linearization 処理中に実行され、プロダクション使用時に stderr にデバッグ情報を漏洩させます。

```
writer.rs:560  eprintln!("DEBUG: Total pages collected: ...")
writer.rs:561  eprintln!("DEBUG: Section 2 objects: ...")
writer.rs:590  eprintln!("DEBUG_PRE_POPULATE: ...")
writer.rs:593  eprintln!("DEBUG_PRE_POPULATE: Failed ...")
writer.rs:780  eprintln!("DEBUG_TOTAL_SIZE_INFO: ...")
writer.rs:934  eprintln!("DEBUG: Page {} reachable count: ...")
writer.rs:1001 eprintln!("DEBUG: doc_reachable count: ...")
writer.rs:1002 eprintln!("DEBUG: shared_objs count: ...")
writer.rs:1112 eprintln!("DEBUG_MISSING_SHARED: ...")
writer.rs:1569 eprintln!("DEBUG_FINALIZE: ...")
writer.rs:1808 eprintln!("DEBUG_REFS: ...")
writer.rs:1932 eprintln!("DEBUG_HINT_TABLE_PAGE: ...")
writer.rs:2010 eprintln!("DEBUG_SHARED_OFFSET: ...")
writer.rs:2040 eprintln!("DEBUG_SEQ1_DELTAS: ...")
writer.rs:2051 eprintln!("DEBUG_SEQ2_DELTAS: ...")
writer.rs:2052 eprintln!("DEBUG_SEQ2_IDS: ...")
```

> [!CAUTION]
> これらは `log::debug!` マクロに置換すべきです。現状では `env_logger` 設定に関係なくユーザーの stderr に内部情報が出力されます。

### 1.2 テストモジュールが `#[cfg(test)]` なしで公開
**影響範囲**: [font/mapping_tests.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/font/mapping_tests.rs)

[mod.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/font/mod.rs#L13) にて：
```rust
pub mod mapping_tests;  // L13: #[cfg(test)] なし
```

内容はテストコードのみ (`#[cfg(test)] mod tests { ... }`) ですが、モジュール宣言自体が `#[cfg(test)]` で保護されていないため、ビルドターゲットに不要なモジュールが含まれ、`pub` 可視性で外部クレートから見えてしまいます。

### 1.3 テストコードに `panic!` が存在
**影響範囲**: [font/reconstruction.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/font/reconstruction.rs)

```
L2053: panic!("GID 1 outline extraction FAILED");
L2057: panic!("Failed to parse reconstructed font: {:?}", e);
```

これらはテストコード内ですが、アサーションマクロ（`assert!` / `assert_eq!`）を使用すべきです。`panic!` はテストフレームワークのメッセージ出力が劣化します。

---

## 2. 重大度: High（早期に対応すべき問題）

### 2.1 ライブラリコード内の `.unwrap()` 呼び出し（非バイナリ）

#### [font/mod.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/font/mod.rs) — 3箇所
```
L910: let df_resolved = df_obj.unwrap().resolve(arena);
L917: let ah = df_array_h.unwrap();
L949: let df_dict = arena.get_dict(dfh.unwrap())?;
```

これらは `parse_descendant_font()` 内で、直前に `is_none()` チェック + `return None` が行われているため実行時に panic することはありませんが、**Option-chaining パターン**に書き換えた方が、後からの変更で guard が壊れた場合にも安全です。

#### [document/page.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/document/page.rs#L95) — 1箇所
```
L95: .find_object(&self.get_attribute("Contents").unwrap())
```

`get_attribute("Contents")` が `None` を返す場合（Contentsのないページ）にパニックします。

### 2.2 Hardcoded File ID (セキュリティ)
**影響範囲**: [writer.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/writer.rs#L530)

```rust
let id_hex = "f00baa42f00baa42f00baa42f00baa42";  // L530, L420
```

PDF の `/ID` は ISO 32000-2 Clause 14.4 にて、文書の一意識別子として規定されています。固定値では：
- 同一 writer を用いた全文書が同一 ID を持つ
- 署名検証やインクリメンタル更新で不整合が起きる
- PDF/A-4 準拠に違反

> [!WARNING]
> ランダム UUID またはファイルハッシュベースの ID 生成に切り替える必要があります。

### 2.3 テストカバレッジが極めて低い

テスト実行結果の分析：

| クレート | テスト数 | 状態 |
|:---|:---:|:---|
| ferruginous-core | 4 | ✅ 全パス |
| ferruginous-render | 3 | ✅ 全パス |
| ferruginous-sdk | 0 | ⚠️ テストなし |
| ferruginous-mcp | 0 | ⚠️ テストなし |
| ferruginous-wasm | 0 | ⚠️ テストなし |
| ferruginous (GUI) | 0 | ⚠️ テストなし |
| fepdf (CLI) | 0 | ⚠️ テストなし |
| ferruginous-macros | 0 | ⚠️ テストなし |

**合計 7 テスト**。88,752 バイトの `writer.rs`、93,607 バイトの `font/mod.rs`、81,813 バイトの `font/reconstruction.rs` など大規模モジュールにテストがほぼ存在しません。ISO 32000-2 準拠を謳うプロジェクトとしては致命的です。

### 2.4 スタブ/未実装の公開API

| 場所 | 内容 |
|:---|:---|
| [sdk/lib.rs:883-885](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/lib.rs#L883-L885) | `upgrade_to_standard()` — `// TODO: Rule-based upgrade logic` のみで即座に `Ok(())` を返す |
| [sdk/lib.rs:919-923](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/lib.rs#L919-L923) | `set_vacuum()`, `set_strip()`, `set_password()` — 全てパラメータ無視の no-op |
| [wasm/lib.rs:35-38](file:///Users/jun/Projects/ferruginous/crates/ferruginous-wasm/src/lib.rs#L35-L38) | `render_page()` — プレースホルダーで常に `Ok(())` |
| [mcp/tools/signature.rs:27](file:///Users/jun/Projects/ferruginous/crates/ferruginous-mcp/src/tools/signature.rs#L27) | `// STUB: Signature verification engine` |
| [arena.rs:255](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/arena.rs#L255) | `// STUB: This will be replaced by the real IR parser in Pass 2` |

> [!IMPORTANT]
> `SaveOptions` の `vacuum`, `strip`, `password` フィールドは公開APIとして文書化されていますが、呼び出しても何も起きません。ユーザーにとっての信頼性問題です。

---

## 3. 重大度: Medium（改善推奨）

### 3.1 Clippy 警告 (7件)

| クレート | 種別 | 件数 |
|:---|:---|:---:|
| ferruginous-core | `collapsible_if` | 4 |
| ferruginous-core | `collapsible_match` | 1 |
| ferruginous-core | `needless_borrows_for_generic_args` | 2 (ただし workspace lint で allow) |
| ferruginous-sdk | `branches_sharing_code` (nursery) | 1 |

すべて軽微ですが、`clippy::pedantic` + `clippy::nursery` を有効にしている以上、CI で clean pass にするべきです。

### 3.2 FIXME コメント (3件)

| 場所 | 内容 |
|:---|:---|
| [refine/mod.rs:327](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/refine/mod.rs#L327) | `Handle::new(0) // FIXME: Real implementation needs access to the arena being built` — `#[allow(dead_code)]` で抑制されている未使用関数 |
| [interpreter/ops/state.rs:100](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/interpreter/ops/state.rs#L100) | `// FIXME: Tell backend about blend mode` |
| [interpreter/ops/xobject.rs:354](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/interpreter/ops/xobject.rs#L354) | `// FIXME: Properly expand indexed images` |

### 3.3 Lab→RGB 変換が仮実装
**影響範囲**: [graphics/mod.rs:35](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/graphics/mod.rs#L35)

```
// TODO(RR-15-EXT): Implement full ICC profile-based Lab-to-sRGB conversion.
```

Lab カラースペースの変換が近似式のため、ICC プロファイルを持つ PDF での色再現に限界があります。

### 3.4 `retag()` 関数が候補を適用しない
**影響範囲**: [sdk/remediation.rs:546-550](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/remediation.rs#L546-L550)

```rust
pub fn retag(doc: &mut Document) -> PdfResult<()> {
    let engine = HeuristicEngine::new();
    let _candidates = engine.infer_structure(doc)?;
    Ok(())
}
```

`infer_structure` で候補を推定するものの、`apply_remediations` を呼ばないため何も変更されません。`PdfDocument::retag_document()` からも同じ `retag()` を呼んでいるため、ユーザーが retag を期待しても何も起きません。

### 3.5 セキュリティ: V5 暗号化のキー導出が非標準
**影響範囲**: [security.rs:101-115](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/security.rs#L101-L115)

`new_v5()` のキー導出が、ユーザーパスワード + オーナーパスワード + FileID の単純 SHA-256 ハッシュになっています。ISO 32000-2 Clause 7.6.4.3.3 では、AES-256 Revision 6 は **PDF 2.0 固有の多段ハッシュ** を使用します。現在の実装はセキュリティ上不十分です。

### 3.6 macOS ハードコードされたフォントパス
**影響範囲**: [document.rs:148-168](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/document.rs#L148-L168)

```rust
let mac_paths = [
    (FallbackFontType::JapaneseSerif, "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc"),
    ...
];
```

Linux/Windows 環境では全てのシステムフォント読み込みが silent fail します。`FERRUGINOUS_RESOURCES` 環境変数にフォールバックしますが、cross-platform サポートとしては不十分です。

---

## 4. 重大度: Low（品質改善）

### 4.1 `#[allow(clippy::...)]` のワークスペース全体への適用

[Cargo.toml:95-121](file:///Users/jun/Projects/ferruginous/Cargo.toml#L95-L121) で **27 種類**の Clippy lint が `allow` されています。`pedantic` + `nursery` を有効にした上で大半を `allow` するのは、lint の効果を大幅に減殺します。

特に懸念：
- `needless_pass_by_value = "allow"` — 所有権の不要なムーブを検出できない
- `cast_precision_loss = "warn"` だが `cast_possible_truncation = "warn"` — 実際には `#[allow]` で個別に抑制される箇所が多い

### 4.2 `SublimatedData::Commands` のフィールド名不整合

[object.rs:221](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/object.rs#L221):
```rust
Commands { items: Vec<sublimation::Command> },
```

しかし使用箇所では `Commands { items: cmds, .. }` と名前 `cmds` でバインドされます。`items` というフィールド名は `commands` に変更した方が自然です。

### 4.3 `find_object()` が O(n) 線形探索
**影響範囲**: [arena.rs:184-192](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/arena.rs#L184-L192)

```rust
pub fn find_object(&self, object: &Object) -> Option<Handle<Object>> {
    let objects = self.inner.objects.read();
    for (i, entry) in objects.iter().enumerate() {
        if &entry.object == object { return Some(Handle::new(i as u32)); }
    }
    None
}
```

オブジェクトプール全体の線形探索です。大規模ドキュメントでは性能ボトルネックになり得ます。`find_object_by_dict_handle()` も同様。

### 4.4 バイナリツール群の `unwrap()` 多用

`src/bin/` 以下のユーティリティバイナリ（`find_page.rs`, `dump_fonts.rs`, `dump_cs.rs` 等）に合計 **30 箇所以上** の `.unwrap()` が存在します。開発ツールとはいえ、不正な PDF を入力した場合にスタックトレースなしで panic します。

### 4.5 `out.pdf`, `out.txt`, `out_check.txt` がリポジトリルートに存在

プロジェクトルートにビルド成果物 / テスト出力が追跡されています。`.gitignore` に追加すべきです。

---

## 5. アーキテクチャ観察

### 5.1 良い点
- **`unsafe` コード完全禁止** (`unsafe_code = "forbid"`) — 極めて模範的
- **Arena パターン** — Handle ベースの参照管理で、GC 不要かつスレッドセーフ
- **3 フェーズパイプライン** (Ingest → Refine → Serialize) — 明確な責務分離
- **ISO Clause アノテーション** — `FromPdfObject` derive マクロに clause 番号を付与
- **Zstd 圧縮** — 大規模ストリームの内部圧縮でメモリ効率化
- **Font Reconstruction** — CFF/TrueType の SFNT ラッピングと仮想 cmap 注入

### 5.2 懸念点
- **writer.rs が 2,255 行 88KB** — 単一ファイルに Linearization + Standard + Incremental + Signature + Hint Table 生成が混在。モジュール分割が急務
- **font/mod.rs が 2,419 行 93KB** — 同様に分割推奨
- **font/reconstruction.rs が 81KB** — 同上
- **テストが事実上ないに等しい** — ワークスペース全体でわずか 7 テスト

---

## 6. サマリーテーブル

| 重大度 | 件数 | 主要項目 |
|:---|:---:|:---|
| 🔴 Critical | 3 | デバッグ eprintln 残留、テストモジュール cfg 欠落、テスト内 panic |
| 🟠 High | 4 | unwrap (ライブラリ)、hardcoded File ID、テストカバレッジ、スタブ API |
| 🟡 Medium | 6 | Clippy 警告、FIXME、Lab 変換、retag 未実装、V5 暗号化、macOS パス |
| 🟢 Low | 5 | allow 過多、フィールド名、O(n) 探索、bin unwrap、成果物残留 |

> [!IMPORTANT]
> 修正は指示があるまで行いません。優先度や対応方針について指示をお願いします。
