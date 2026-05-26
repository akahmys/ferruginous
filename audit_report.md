# Ferruginous コードベース監査レポート (第2回)

> [!NOTE]
> 本レポートは `2026-05-26` 時点のコードベース全体を対象とした再監査です。
> 最新コミット: `d220269` (chore: strictly comply with ISO 32000-2 & resolve critical/high/low audit report issues)
> ビルド: ✅ 成功 / テスト: ✅ 全件パス (7件) / unsafe: ✅ 使用なし (`forbid` 設定済)

---

## 前回からの修正状況

| 前回の指摘 | 重大度 | 状態 |
|:---|:---:|:---:|
| `writer.rs` の `eprintln!("DEBUG: ...")` 18箇所 | 🔴 Critical | ✅ **修正済** — `log::debug!` に変換 |
| `mapping_tests` に `#[cfg(test)]` 欠落 | 🔴 Critical | ✅ **修正済** — L13 に `#[cfg(test)]` 追加 |
| テスト内 `panic!` (reconstruction.rs) | 🔴 Critical | ⚠️ **未修正** → Medium に降格（テストコード限定のため） |
| `font/mod.rs` ライブラリ内 `.unwrap()` | 🟠 High | ✅ **修正済** — 0件に |
| `page.rs:95` の `.unwrap()` | 🟠 High | ✅ **修正済** — `contents_handles()` に書き換え |
| Hardcoded File ID `f00baa42...` | 🟠 High | ✅ **修正済** — `generate_file_id()` でハッシュベース生成 |
| テストカバレッジ不足 | 🟠 High | ⚠️ **未修正** — 依然7テストのみ |
| スタブAPI (vacuum/strip/password) | 🟠 High | ⚠️ **未修正** |
| Clippy 警告 | 🟡 Medium | ⚠️ **微増** — 7件 → 10件 (新コード追加に伴い3件増加) |
| FIXME コメント | 🟡 Medium | ⚠️ **未修正** — 3件残存 |
| `retag()` 未実装 | 🟡 Medium | ⚠️ **未修正** |
| V5 暗号化の非標準キー導出 | 🟡 Medium | ⚠️ **未修正** |
| Lab→RGB 仮実装 | 🟡 Medium | ⚠️ **未修正** |
| macOS ハードコードパス | 🟡 Medium | ⚠️ **未修正** → Low に降格（設計意図による） |
| `find_object()` O(n) 探索 | 🟢 Low | ⚠️ **未修正** |
| bin ツールの `unwrap()` 多用 | 🟢 Low | ⚠️ **未修正** |
| lint allow 過多 | 🟢 Low | ⚠️ **未修正** |

> [!TIP]
> 前回 Critical 3件中 2件、High 4件中 3件が修正されました。指摘事項の合計は **18件→13件** に改善しています。

---

## 1. 重大度: Critical（即座に対応すべき問題）

**なし** ✅

---

## 2. 重大度: High（早期に対応すべき問題）

### 2.1 テストカバレッジが極めて低い（前回から変化なし）

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

**合計 7 テスト**。`writer.rs` (88KB)、`font/mod.rs` (93KB)、`font/reconstruction.rs` (81KB) 等にテストなし。

### 2.2 スタブ/未実装の公開API（前回から変化なし）

| 場所 | 内容 |
|:---|:---|
| [sdk/lib.rs:883-885](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/lib.rs#L883-L885) | `upgrade_to_standard()` — `// TODO` のみで即 `Ok(())` |
| [sdk/lib.rs:919-923](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/lib.rs#L919-L923) | `set_vacuum()`, `set_strip()`, `set_password()` — no-op |
| [mcp/tools/signature.rs:27](file:///Users/jun/Projects/ferruginous/crates/ferruginous-mcp/src/tools/signature.rs#L27) | `// STUB: Signature verification engine` |

---

## 3. 重大度: Medium（改善推奨）

### 3.1 Clippy 警告 (10件、前回+3)

| クレート | 種別 | 件数 | 状態 |
|:---|:---|:---:|:---:|
| ferruginous-core | `collapsible_if` | 4 | 前回同様 |
| ferruginous-core | `collapsible_match` | 1 | 前回同様 |
| ferruginous-core | `needless_borrows_for_generic_args` | 2 | 前回同様 |
| ferruginous-core | `map_entry` | 1 | 🆕 新規 ([document.rs:691](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/document.rs#L691)) |
| ferruginous-core | `manual_strip` | 1 | 🆕 新規 ([metadata.rs:140](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/metadata.rs#L140)) |
| ferruginous-sdk | `branches_sharing_code` | 1 | 前回同様 ([writer.rs:1493](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/writer.rs#L1493)) |

### 3.2 FIXME / TODO / STUB コメント (8件残存)

| 場所 | 内容 |
|:---|:---|
| [refine/mod.rs:327](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/refine/mod.rs#L327) | `Handle::new(0) // FIXME: Real implementation needs access to the arena` |
| [interpreter/ops/state.rs:100](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/interpreter/ops/state.rs#L100) | `// FIXME: Tell backend about blend mode` |
| [interpreter/ops/xobject.rs:354](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/interpreter/ops/xobject.rs#L354) | `// FIXME: Properly expand indexed images` |
| [graphics/mod.rs:35](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/graphics/mod.rs#L35) | `// TODO(RR-15-EXT): ICC profile-based Lab-to-sRGB` |
| [sdk/lib.rs:884](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/lib.rs#L884) | `// TODO: Rule-based upgrade logic` |
| [arena.rs:255](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/arena.rs#L255) | `// STUB: real IR parser in Pass 2` |
| [resurrection.rs:114](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/object/sublimation/resurrection.rs#L114) | `// STUB: Dash patterns not yet recovered` |
| [resurrection.rs:135](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/object/sublimation/resurrection.rs#L135) | `// STUB: Array and Dictionary recursion` |

### 3.3 ライブラリコード内の `.unwrap()` (新規発見)

#### [sublimation/parser.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/object/sublimation/parser.rs) — 2箇所

```rust
L667: let key_token = lexer.next_token().unwrap();
L673: let val_token = lexer.next_token().unwrap();
```

`parse_ir_dict()` 内。直前の `lexer.peek()` で EOF チェックはありますが、peek 後に next が失敗するケース（IO エラー等）でパニックします。

#### [obj_stm.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/obj_stm.rs) — 1箇所

```rust
L53: write!(full_data, "{id} {offset} ").unwrap();
```

`Vec<u8>` への `write!` なので実際には失敗しませんが、RR-15 Rule 2 (No-Panic Invariance) に厳密には違反です。

#### [writer.rs](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/writer.rs) — 1箇所

```rust
L2077: let last_id = self.id_map[outline_exclusive.last().unwrap()];
```

`outline_exclusive.is_empty()` チェックの内側なので実行時に安全ですが、`.unwrap()` パターンはガード変更時に脆弱です。

### 3.4 `retag()` 関数が候補を適用しない（前回から変化なし）
**影響範囲**: [sdk/remediation.rs:546-550](file:///Users/jun/Projects/ferruginous/crates/ferruginous-sdk/src/remediation.rs#L546-L550)

```rust
pub fn retag(doc: &mut Document) -> PdfResult<()> {
    let engine = HeuristicEngine::new();
    let _candidates = engine.infer_structure(doc)?;  // 推定するが適用しない
    Ok(())
}
```

### 3.5 V5 暗号化のキー導出が非標準（前回から変化なし）
**影響範囲**: [security.rs:101-115](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/security.rs#L101-L115)

ISO 32000-2 Clause 7.6.4.3.3 では多段ハッシュを要求しますが、単純 SHA-256 のまま。

### 3.6 テスト内 `panic!`（Critical から降格）
**影響範囲**: [font/reconstruction.rs:2053,2057](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/font/reconstruction.rs#L2053)

テストコード限定のため Medium に降格。`assert!` マクロに置き換え推奨。

### 3.7 Lab→RGB 変換が仮実装（前回から変化なし）
**影響範囲**: [graphics/mod.rs:35](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/graphics/mod.rs#L35)

---

## 4. 重大度: Low（品質改善）

### 4.1 `find_object()` が O(n) 線形探索（前回から変化なし）
**影響範囲**: [arena.rs:184-192](file:///Users/jun/Projects/ferruginous/crates/ferruginous-core/src/arena.rs#L184-L192)

### 4.2 バイナリツール群の `unwrap()` 多用（前回から変化なし）
`src/bin/` 以下に **30箇所以上**。

### 4.3 `#[allow(clippy::...)]` ワークスペース全体への過剰適用（前回から変化なし）
[Cargo.toml:95-121](file:///Users/jun/Projects/ferruginous/Cargo.toml#L95-L121) で27種類の lint を `allow`。

### 4.4 macOS ハードコードフォントパス（Medium から降格）
`FERRUGINOUS_RESOURCES` 環境変数 + `assets/fonts/` フォールバックが設計意図として存在するため Low に再分類。

---

## 5. サマリーテーブル

| 重大度 | 前回 | 今回 | 増減 |
|:---|:---:|:---:|:---:|
| 🔴 Critical | 3 | **0** | -3 ✅ |
| 🟠 High | 4 | **2** | -2 ✅ |
| 🟡 Medium | 6 | **7** | +1 (新規 unwrap 発見 + Clippy 増) |
| 🟢 Low | 5 | **4** | -1 |
| **合計** | **18** | **13** | **-5** |

> [!IMPORTANT]
> Critical が 0 になったのは大きな進歩です。残る最重要課題は **テストカバレッジ** と **スタブ API** です。修正は指示があるまで行いません。
