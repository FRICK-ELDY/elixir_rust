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
| **1.7.5** | **GameWorld から描画データ取得経路の実装**: 描画スレッド内で ResourceArc&lt;GameWorld&gt; を read し、get_frame_metadata（HUD用）および描画用スナップショット（スプライト・パーティクル等）を取得して renderer に渡す |
| **1.7.6** | **Elixir 起動フローとの接続**: Application または GameEvents の init で「描画開始」NIF を 1 回呼び出し、iex -S mix 起動時にウィンドウが開くようにする |
| **1.7.7** | **game_window クレートの廃止**: workspace から game_window を削除。ビルド・CI・README を game_native 単体で動くように更新する |
| **1.7.8** | **Windows 動作確認とドキュメント更新**: Windows で iex -S mix から描画まで一通り動作確認。FOLDER_CONNECTIONS.md と ARCHITECTURE.md を 1.7 完了後の構成に更新する |

**推奨順序**: 1.7.1 → 1.7.2 → 1.7.3 → 1.7.4 → 1.7.5 → 1.7.6 → 1.7.7 → 1.7.8。

---

## 1.7.1 アーキテクチャとデータ共有方式（確定）

**実施日**: 2025-02-25

### 決定事項サマリ

| 項目 | 決定内容 |
|------|----------|
| **描画スレッドの spawn 方法** | `std::thread::spawn` で専用スレッドを起動。NIF `start_render_thread(world_ref)` が `ResourceArc<GameWorld>` を clone してスレッドに渡す。`start_rust_game_loop` と同様のパターン。 |
| **スレッドのライフサイクル** | winit の `EventLoop::run()` がブロックするため、spawn したスレッド内で実行。戻り値は不要。 |
| **GameWorld とのデータ共有** | `ResourceArc<GameWorld>` を clone して描画スレッドへ渡す。描画スレッド内では `world.0.read()` で `RwLockReadGuard` を取得し、スナップショットを構築。 |
| **スナップショット取得方式** | read でロック取得 → 必要なデータを `RenderSnapshot` にコピー → ロック解放 → ロック外で wgpu 描画。read の保持時間を最小化する。 |

---

### 1. 描画スレッドの spawn 方法（決定）

#### 1.1 スレッド起動パターン

`start_rust_game_loop` と同様、NIF から `std::thread::spawn` を用いて専用スレッドを起動する。

```rust
// 例: nif/render_nif.rs（1.7.4 で実装）
#[rustler::nif]
pub fn start_render_thread(world: ResourceArc<GameWorld>) -> NifResult<Atom> {
    let world_clone = world.clone();
    thread::spawn(move || {
        if let Err(e) = std::panic::catch_unwind(move || {
            run_render_thread(world_clone);  // winit EventLoop::run がブロック
        }) {
            eprintln!("Render thread panicked: {:?}", e);
        }
    });
    Ok(ok())
}
```

#### 1.2 起動タイミングと引数

| 項目 | 内容 |
|------|------|
| **起動タイミング** | GameEvents の init（または Application の start）で、`start_rust_game_loop` の後、または並列に 1 回だけ呼ぶ。 |
| **引数** | `world_ref: ResourceArc<GameWorld>` のみ。Elixir 側の `world_ref` をそのまま渡す。 |
| **戻り値** | `:ok` を返す。スレッドはバックグラウンドで動作し、BEAM はブロックしない。 |

#### 1.3 パニックハンドリング

`thread::spawn` で起動したスレッドがパニックすると、NIF 呼び出し元（Elixir 側）には通知されず、デフォルトではパニックが握りつぶされる。デバッグと堅牢性のため、`std::panic::catch_unwind` でパニックを捕捉し、ログ出力する（上記コード例参照）。コンパイル時に `UnwindSafe` 関連のエラーが出る場合は `AssertUnwindSafe` でラップする。

#### 1.4 winit EventLoop の配置

- `EventLoop::run()` はブロッキングであるため、spawn したスレッド内でのみ呼ぶ。
- BEAM スレッドやゲームループスレッドからは呼ばない。
- 描画スレッドはウィンドウを閉じるまで生き続ける。終了時はプロセスごと終了する前提。

---

### 2. GameWorld と描画スレッド間のデータ渡し（決定）

#### 2.1 共有モデル

```
[ゲームループスレッド]                    [描画スレッド]
        │                                      │
        │ world.0.write()                      │ world.0.read()
        │   physics_step_inner                 │   → RenderSnapshot 構築
        │   drain_frame_events_inner           │   → ロック解放
        │   (約 1–2 ms / フレーム)             │   → ロック外で wgpu 描画
        ▼                                      ▼
        └────────── RwLock<GameWorldInner> ────┘
```

- **ゲームループ**: `write()` で排他ロック。60 Hz、フレームあたり約 1–2 ms 想定。
- **描画スレッド**: `read()` で共有ロック。フレームごとに短時間だけロックを保持し、スナップショットをコピーした後に解放。

#### 2.2 スナップショット取得フロー

1. **ロック取得**: `let guard = world.0.read()?`
2. **スナップショット構築**: `guard` を参照して `RenderSnapshot` に必要なデータをコピー（player, enemies, bullets, particles, items, boss 等）
3. **ロック解放**: `guard` がスコープを抜けて `drop` → read ロック解放
4. **描画**: ロック外で `RenderSnapshot` を使って wgpu 描画・egui HUD 描画

#### 2.3 RenderSnapshot の内容（確定）

| データ種別 | 取得元（GameWorldInner） | 用途 |
|------------|--------------------------|------|
| スプライト（player, enemies, bullets, boss） | `get_render_data` 相当のロジック | wgpu スプライト描画 |
| パーティクル | `get_particle_data` 相当 | wgpu パーティクル描画 |
| アイテム | `get_item_data` 相当 | wgpu アイテム描画 |
| 障害物 | `get_obstacle_data` 相当（game_window に存在） | 背景・当たり判定表示 |
| カメラオフセット | `camera_offset` 相当 | 描画位置補正 |
| HUD メタデータ | `get_frame_metadata` 相当（HP, スコア, 敵数, 弾数, レベルアップ等） | egui HUD |
| FPS 用 | `last_frame_time_ms` 等 | デバッグ表示 |

※ 既存の `get_render_data`, `get_particle_data`, `get_item_data`, `get_frame_metadata` のロジックを、描画スレッド内の `build_render_snapshot(w: &GameWorldInner) -> RenderSnapshot` に集約する方針。

#### 2.4 ロック競合の考慮

| 考慮点 | 対策 |
|--------|------|
| **read の保持時間** | スナップショット構築のみでロックを保持。wgpu 描画はロック解放後に実行。 |
| **write vs read** | ゲームループが write 中は read はブロック。1 フレームあたり 1–2 ms 程度を想定し、描画フレームレートへの影響を許容範囲とする。 |
| **read vs read** | 複数 read は並行可。現状は描画スレッドのみが read するため問題なし。 |

#### 2.5 ResourceArc の扱い

- `ResourceArc<GameWorld>` は `Clone` 可能で、参照カウント方式で複数スレッドから共有される。
- Elixir の `world_ref` が ResourceArc を保持。create_world 時に 1 つ生成され、ゲームループ・描画スレッド・NIF 呼び出しで clone される。
- プロセス終了時に ResourceArc がすべて drop され、GameWorld が解放される。

---

### 3. 実装フェーズへの受け渡し

| 1.7.x | 本決定の反映 |
|-------|--------------|
| **1.7.4** | `start_render_thread` NIF と `run_render_thread` の骨組み。thread::spawn と winit EventLoop の配置。 |
| **1.7.5** | `RenderSnapshot` 型の定義、`build_render_snapshot` の実装、描画ループ内での read → スナップショット → 描画の接続。 |

---

## 1.7.7 game_window クレートの廃止（実施）

**実施日**: 2026-02-25

### 実施内容

- `native/Cargo.toml` の workspace members から `game_window` を削除。
- `native/game_window/` 配下のクレート本体（`Cargo.toml`, `src/main.rs`, `renderer`, `asset`, `audio`）を削除。
- `bin/start.bat` を `cargo run -p game_window` から `iex -S mix` 起動へ変更（統合起動）。
- `README.md` の起動手順を `iex -S mix` に更新。

### 補足

- このリポジトリには現時点で `.github/workflows/` が存在しないため、CI 設定の更新対象はなし。

---

## 1.7.8 Windows 動作確認とドキュメント更新（実施）

**実施日**: 2026-02-25

### 動作確認結果

- Windows で `iex -S mix` から起動し、ゲームとしてプレイ可能であることを確認。
- `game_window` なしの構成で、`game_native` 側の描画スレッド起動・ウィンドウ表示・描画更新が成立。
- 1.7 の到達目標（単一プロセス起動での描画統合）は達成とする。

### 確認観点（1.7.8）

| 観点 | 結果 |
|------|------|
| `iex -S mix` 単体起動でウィンドウ表示 | OK |
| 描画更新と入力反映 | OK |
| ゲーム進行（プレイ可能性） | OK |
| `game_window` 非依存での実行 | OK |

### 更新ドキュメント

- `docs/06_system_design/FOLDER_CONNECTIONS.md` を 1.7 完了後の構成（`game_window` 廃止、`game_native` 集約）に合わせて更新。
- `docs/06_system_design/ARCHITECTURE.md` を単一プロセス + NIF 描画スレッド構成に合わせて更新。
- 本書（`STEPS_RENDER_INTEGRATION.md`）に 1.7.8 の実施記録を追記。

### 補足

- macOS はメインスレッド制約があるため、現設計のままでは未対応。対応時は本書の「macOS 対応について（設計修正予定）」に従って再設計する。

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

- **Elixir 側**: `lib/app`、`lib/engine`、`lib/games` のアーキテクチャに大きな変更はありません。`NifBridge` に「描画開始」用 NIF を追加し、`Application` または `GameEvents` の起動時に 1 回呼び出す変更のみです。

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

### macOS 対応について（設計修正予定）

**現状の制約**: `std::thread::spawn` で描画スレッドを起動する現在の設計は、macOS では動作しない。macOS では UI 関連処理（winit の EventLoop を含む）は**メインスレッド**で実行する必要があり、サブスレッドから起動するとパニックまたはクラッシュする可能性が高い。

**Windows での対応**: `EventLoopBuilderExtWindows::with_any_thread(true)` により、非メインスレッドからの EventLoop 作成を許可している（現状 Windows のみ対応）。

**macOS 対応時の設計修正案**:
- アプリ起動方法を再設計し、Rust 側がメインスレッドを確保して winit を初期化する構成にする。
- 例: Rust バイナリをラッパーとして用意し、メインスレッドで winit を起動しつつ、別スレッドで BEAM（Elixir VM）を起動する。

macOS / Linux 対応時には本ドキュメントの設計を修正する。

---

### 再描画タイミング（設計修正予定）

**現状の制約**: `RedrawRequested` イベントハンドラの末尾で `window.request_redraw()` を無条件に呼び出すと、CPU 100% 消費のビジーループが発生する。1.7.4 の骨組みでは動作確認を優先しており、この実装のままとなっている。

**対応方針**: 再描画は、ゲーム状態が更新されたときなど、画面を更新する必要がある場合にのみリクエストするように変更する。1.7.5 以降で GameWorld read と描画の接続を行う際に対応予定。

**対策案**:
- `EventLoopProxy` を使い、ゲームループスレッド側から再描画を要求する
- `ControlFlow::WaitUntil` を使い、次回の描画タイミングまで待機する

---

## 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [FOLDER_CONNECTIONS.md](../../06_system_design/FOLDER_CONNECTIONS.md) | フォルダ接続関係 |
| [STEPS_RUST_LIB.md](./STEPS_RUST_LIB.md) | 1.6 のクレート構成 |
