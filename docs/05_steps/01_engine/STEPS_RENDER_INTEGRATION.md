# 1.7 描画統合（game_window → game_native）

**所属**: [STEPS_ALL.md](../STEPS_ALL.md) 1章 エンジン構築 の 1.7 節。

**目的**: game_window バイナリを廃止し、renderer / winit / wgpu を game_native に統合する。NIF が描画スレッドを spawn し、`iex -S mix` 単一プロセスで wgpu 描画を実行する。

**前提**: 1.6 Rust lib 分割・整理が完了していること。

---

## 概要

| 項目 | 内容 |
|------|------|
| **アーキテクチャ** | NIF が描画スレッドを spawn。同一プロセス（iex -S mix）内で winit のイベントループ + wgpu 描画を実行 |
| **描画対象** | NIF 内の GameWorld（get_frame_metadata 等）と Elixir 側のシーン・UI 状態 |
| **状態管理** | 当面は案 C（GameWorld は Rust 側で保持）。将来案 A（Elixir で保持）をオーバーヘッド確認のため検証 |
| **ビルド対象** | まずは Windows。将来的にクロスビルド対応 |

### 状態管理の案（案 A / 案 C）

| 案 | 概要 |
|----|------|
| **案 C**（当面採用） | GameWorld を Rust（NIF）側に保持。物理・衝突・AI は NIF 内で実行。Elixir は描画コマンドやシーン・UI 状態を渡す。現状の設計を維持。 |
| **案 A**（将来検証） | GameWorld を Elixir 側に移す。毎フレーム Elixir → Rust で状態を送り、物理は NIF で処理して結果を Elixir に返す。オーバーヘッド（シリアライズ／NIF 境界）の計測後に採用可否を判断する。 |

---

## 1.7 節 全体ロードマップ（1.7.1〜1.7.8）

| 項 | 目標 |
|----|------|
| **1.7.1** | **アーキテクチャとデータ共有方式の確定**: 描画スレッドの spawn 方法、GameWorld と描画スレッド間のデータ渡し（ResourceArc の read でスナップショット取得等）を文書化・決定する |
| **1.7.2** | **game_native に renderer を追加**: game_window の renderer/ を game_native に移動。game_core 依存を維持し、型・定数は game_core を参照する |
| **1.7.3** | **game_native に asset / audio を追加**: game_window の asset/ と audio を game_native に移動。アセットパスは環境変数（GAME_ASSETS_ID 等）で解決する |
| **1.7.4** | **描画スレッド spawn と winit 統合**: NIF（例: start_render_thread）で描画用スレッドを spawn。そのスレッドで winit の EventLoop・ウィンドウ作成・wgpu 初期化の骨組みを実装する |
| **1.7.5** | **GameWorld から描画データ取得経路の実装**: 描画スレッド内で ResourceArc&lt;GameWorld&gt; を read し、get_frame_metadata および描画用スナップショット（スプライト・パーティクル・HUD）を取得して renderer に渡す |
| **1.7.6** | **Elixir 起動フローとの接続**: Application または GameEvents の init で「描画開始」NIF を 1 回呼び出し、iex -S mix 起動時にウィンドウが開くようにする |
| **1.7.7** | **game_window クレートの廃止**: workspace から game_window を削除。ビルド・CI・README を game_native 単体で動くように更新する |
| **1.7.8** | **Windows 動作確認とドキュメント更新**: Windows で iex -S mix から描画まで一通り動作確認。FOLDER_CONNECTIONS.md と ARCHITECTURE.md を 1.7 完了後の構成に更新する |

**推奨順序**: 1.7.1 → 1.7.2 → 1.7.3 → 1.7.4 → 1.7.5 → 1.7.6 → 1.7.7 → 1.7.8。

---

## 最終アーキテクチャ（1.7 完了後）

### プロセス・スレッド構成

- **単一プロセス**: `iex -S mix` のみ。game_window バイナリは廃止され、描画は NIF から spawn したスレッド内で行う。
- **BEAM メインスレッド**: GameEvents GenServer が NIF を呼ぶ（create_world, physics_step, drain_frame_events, get_frame_metadata, start_rust_game_loop, **描画開始 NIF**）。
- **Rust ゲームループスレッド**: 既存の `start_rust_game_loop` で spawn されたスレッド。60 Hz で physics_step と drain_frame_events を実行し、Elixir に frame_events を送る。
- **Rust 描画スレッド**: 新規。NIF で spawn し、winit の EventLoop + wgpu でウィンドウ表示・入力・描画を担当。毎フレーム `ResourceArc<GameWorld>` を read して描画用データを取得する。

### データフロー

```
Elixir (GameEvents)  ──NIF──► game_native (NIF)
                                    │
                                    ├─ create_world / start_rust_game_loop / start_render_thread
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
            [ゲームループスレッド]              [描画スレッド]
                    │                               │
                    │ physics_step_inner            │ winit event_loop
                    │ drain_frame_events            │ wgpu 描画
                    │ world.write()                 │ world.read() → スナップショット
                    │ frame_events → Elixir         │ get_frame_metadata 相当で HUD
                    ▼                               ▼
            ResourceArc<GameWorld>  ◄────共有────► ResourceArc<GameWorld>
```

- **GameWorld**: Elixir の ResourceArc として 1 つだけ存在。ゲームループスレッドが write（physics_step）、描画スレッドが read（描画用スナップショット取得）。状態管理は**案 C**（Rust 側で保持）のまま。

### クレート・フォルダ構成（1.7 完了後）

```
native/
├── Cargo.toml                 # workspace（game_window を削除）
├── xtask/
├── game_core/                 # 変更なし：共通ロジック・定数・物理
├── game_native/               # 描画統合後
│   ├── Cargo.toml             # winit, wgpu, egui, rodio 等を追加
│   └── src/
│       ├── lib.rs
│       ├── world/
│       ├── game_logic/
│       ├── nif/               # start_render_thread 等の描画 NIF を追加
│       ├── renderer/          # game_window から移動
│       ├── asset/             # game_window から移動
│       ├── audio.rs           # game_window から移動
│       └── render_thread.rs   # 描画スレッドのエントリ（winit run 等）
└── (game_window は廃止)
```

- **Elixir 側**: lib/app, lib/engine, lib/games は変更なし。NifBridge に「描画開始」用 NIF を追加し、Application または GameEvents の起動時に 1 回呼ぶ。

### 起動シーケンス（1.7 完了後）

1. `iex -S mix` で BEAM 起動。
2. Application が GameEvents 等を起動。
3. GameEvents の init で `create_world` → `create_game_loop_control` → `start_rust_game_loop`（既存）。
4. **新規**: 同一 init または Application で `start_render_thread(world_ref)` を 1 回呼ぶ。NIF が描画スレッドを spawn し、ウィンドウが表示される。
5. ゲームループスレッドが 60 Hz で物理・イベントを回し、描画スレッドが winit のフレームごとに GameWorld を read して描画。

### 描画対象の扱い

| データ | 取得元 | 用途 |
|--------|--------|------|
| スプライト・パーティクル・弾・敵・プレイヤー位置 | GameWorld を read して NIF 内の get_render_data 相当 | wgpu スプライト描画 |
| HUD・敵数・HP・レベルアップ・ボス情報 | get_frame_metadata 相当を描画スレッド内で取得 | egui HUD |
| シーン・UI 状態（Elixir 側） | 将来拡張: Elixir から NIF に渡すか、同一フレームの get_frame_metadata に含める | メニュー・ポーズ等 |

当面は NIF 内の GameWorld のみで描画。Elixir 側のシーン・UI 状態は、必要に応じて get_frame_metadata の拡張や別 NIF で渡す形で後から対応する。

### 注意事項

- **プラットフォーム**: まずは Windows で動作確認。macOS / Linux は 1.7.8 以降で検証。
- **winit のスレッド**: winit の `EventLoop::run()` はブロックするため、必ず描画用の専用スレッドで実行する。
- **ResourceArc のスレッド安全性**: `RwLock` でゲームループが write、描画が read するため、ロックの競合を抑える（描画側は read のみ・短時間で済むようにする）。

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [FOLDER_CONNECTIONS.md](../../06_system_design/FOLDER_CONNECTIONS.md) | フォルダ接続関係 |
| [STEPS_RUST_LIB.md](./STEPS_RUST_LIB.md) | 1.6 のクレート構成 |
