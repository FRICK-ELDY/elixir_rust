# ADR: Shared Memory + Thread Policy

- Status: Accepted
- Date: 2026-02-26
- Scope: `native/game_native` `lib/engine` `lib/app`

## Context

敵・弾・パーティクルなどの高頻度データを Elixir ヒープに毎フレーム複製すると、
NIF 境界の変換コストと GC 負荷が増え、60fps 維持が難しくなる。

一方で Elixir 側の強み（プロセス監視、復旧、分散制御）を失う設計は避ける必要がある。

## Decision

1. **Shared Memory**
   - ゲーム状態の実体は Rust 側 `GameWorld`（`ResourceArc<RwLock<GameWorldInner>>`）で保持する。
   - Elixir は `Resource` ハンドル参照を保持し、巨大配列をヒープ上に常駐させない。

2. **Zero-copy の定義**
   - 本プロジェクトの「ゼロコピー」は **Resource 参照中心** を意味する。
   - 毎フレームの `Vec -> Elixir list/tuple` 全量転送は行わない。

3. **Threading model (`C_pragmatic`)**
   - 計算スレッド: `World` の高頻度 write 主体。
   - 描画スレッド: read lock + snapshot 構築のみ。
   - NIF スレッド: 制御コマンドと軽量 query 中心。
   - 音スレッド: World 直接更新ではなくコマンド駆動。

4. **NIF API 分類**
   - `control`: World 変更やスレッド制御（高頻度 write を許可）
   - `query_light`: 軽量な状態観測（read only）
   - `snapshot_heavy`: セーブ/デバッグ等の明示操作時のみ（毎フレーム禁止）

## Consequences

- 高頻度データの境界往復を抑制でき、フレーム予算を守りやすい。
- Elixir は監視・復旧・ルーム管理の中核を維持できる。
- API レビュー時は「60fps 制約か、運用制約か」で責務を判定できる。

## Guardrails

- 毎フレームに許可するのは `query_light` のみ。
- `snapshot_heavy` はセーブ/ロード/デバッグなど明示操作時のみ呼ぶ。
- lock 競合は計測し、閾値超過時に警告を出す（詳細は実装メトリクスに従う）。
