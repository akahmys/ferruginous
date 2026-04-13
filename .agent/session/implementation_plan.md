# Git 運用方針の刷新と GitHub 同期計画

ユーザー様のご提案に基づき、プロジェクトの健全な成長と「メインブランチの安定性」を両立するため、今後の開発を **「ブランチベース」** へと正式に移行します。

## Proposed Changes

### [Component] Branch Strategy (基本方針)

1.  **main ブランチ**: 
    - 常に「ビルド可能」かつ「既存テストをパスする」安定版。
    - ユーザーによる最終承認（Merge）を受けた成果物のみを保持。
2.  **feat/fix ブランチ**:
    - 特定のフェーズ（例: Phase 18）や機能、バグ修正ごとに作成。
    - AI はこのブランチ上で大胆な診断や実装変更を行い、完了後に PR（またはマージ提案）を作成。

### [Component] Execution Steps (本ターンの作業)

#### 1. 規約の更新
- **[PLANNING_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/PLANNING_PROTOCOL.md)** に「第 6 章: バージョン管理規約」を追記し、ブランチ戦略を明文化。

#### 2. 現在の修正の永続化 (to main)
- 現在の「インフラ整備・規約リファクタリング」は、全フェーズの共通基盤であるため、**一度 `main` ブランチにコミット・プッシュ** します。
- これにより、プロジェクトの基底状態（Baseline）を最新化します。

#### 3. 次期フェーズの開始 (to branch)
- Phase 18（日本語描画）の開始時に、新ブランチ `feat/phase18-multibyte-diagnosis` を作成。
- 以降の診断・修正はこのブランチ上で行い、`main` の安定性を保護します。

---

## Verification Plan

### Manual Verification
- `git checkout -b` コマンドが正常に動作し、ブランチが作成されることを確認。
- [PLANNING_PROTOCOL.md](file:///Users/jun/Documents/Project/Ferruginous/.agent/protocols/PLANNING_PROTOCOL.md) に追記されたルールが、ユーザー様の意図と合致しているか再確認。

---

## Open Questions

- **ブランチの命名**: 基本は `feat/` (機能) または `fix/` (修正) としますが、Phase 単位で `phase18/` のようなプレフィックスを使用する方が管理しやすいでしょうか？
