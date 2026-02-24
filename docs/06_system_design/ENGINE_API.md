# エンジン API 設計（安定化）

**対象**: Step 37（エンジン API の安定化）  
**根拠**: [NEXT_STEPS.md](../04_roadmap/NEXT_STEPS.md)、[GAME_INTERFACE.md](./GAME_INTERFACE.md)

---

## 1. 方針

**ゲームは `Engine` モジュール経由でのみエンジンとやり取りする。**

- `Game.NifBridge` を直接呼び出さない
- `Engine.SceneManager` を直接呼び出さない（シーン遷移は update の戻り値で表現）
- エンジン内部（GameLoop、FrameCache 等）は必要に応じて NifBridge / SceneManager を直接利用してよい

この方針により、将来 NIF の実装が変わっても、ゲームコードの変更を最小限に抑えられる。

---

## 2. モジュール構成

| モジュール | 役割 | ゲームから呼ぶ？ |
|-----------|------|-----------------|
| `Engine` | 安定化された公開 API（本ドキュメントの対象） | 〇 |
| `Engine.Game` | ゲームがエンジンに提供する behaviour | 実装する |
| `Engine.SceneBehaviour` | シーンが実装する behaviour | 実装する |
| `Game.NifBridge` | Rust NIF のラッパー | 呼ばない |
| `Engine.SceneManager` | シーンスタック管理 | 呼ばない |
| `Engine.GameLoop` | 60Hz ループ orchestration | 呼ばない |

---

## 3. 公開 API リファレンス

### 3.1 ゲームから利用する API（World 操作）

context の `world_ref` を受け取り、ワールドに対する操作を行う。

| 関数 | 説明 |
|------|------|
| `Engine.spawn_enemies(world_ref, kind, count)` | 通常敵をスポーン。`kind`: `:slime` \| `:bat` \| `:golem` |
| `Engine.spawn_elite_enemy(world_ref, kind, count, hp_multiplier)` | エリート敵をスポーン |
| `Engine.spawn_boss(world_ref, kind)` | ボスをスポーン。`kind`: `:slime_king` \| `:bat_lord` \| `:stone_golem` |
| `Engine.get_enemy_count(world_ref)` | 現在の敵数を返す |
| `Engine.is_player_dead?(world_ref)` | プレイヤー死亡判定 |
| `Engine.get_level_up_data(world_ref)` | `{exp, level, level_up_pending, exp_to_next}` を返す |
| `Engine.skip_level_up(world_ref)` | 武器選択をスキップしてレベルアップ待機を解除 |

### 3.2 Step 44: ルーム管理（マルチプレイ基盤）

| 関数 | 説明 |
|------|------|
| `Engine.start_room(room_id)` | 新規ルームを起動。`{:ok, pid}` / `{:error, :already_started}` |
| `Engine.stop_room(room_id)` | ルームを終了。`{:error, :not_found}` はルーム不在時 |
| `Engine.list_rooms()` | アクティブなルーム ID のリスト |
| `Engine.get_loop_for_room(room_id)` | ルームの GameLoop pid。`{:ok, pid}` / `:error` |

### 3.3 エンジン内部用（ゲームは通常呼ばない）

GameLoop が利用。将来の拡張やカスタムループ実装のため文書化する。

| 関数 | 説明 |
|------|------|
| `Engine.create_world()` | ワールドを生成。GameLoop の init で呼ばれる |
| `Engine.physics_step(world_ref, delta_ms)` | 物理演算を1ステップ実行 |
| `Engine.set_player_input(world_ref, dx, dy)` | プレイヤー入力を設定 |
| `Engine.drain_frame_events(world_ref)` | フレームイベントを取得（EventBus に broadcast） |
| `Engine.get_frame_metadata(world_ref)` | HUD 等のメタデータを取得 |
| `Engine.add_weapon(world_ref, weapon_name)` | 武器を追加（atom または string） |
| `Engine.get_weapon_levels(world_ref)` | 装備中の武器スロット情報を取得 |

### 3.4 シーン操作（GameLoop が transition で使用）

ゲームは **直接呼ばない**。update の戻り値で遷移意図を伝える。

| 戻り値 | 意味 |
|--------|------|
| `{:continue, state}` | 継続、state を更新 |
| `{:continue, state, opts}` | 継続、context_updates など opts を付与 |
| `{:transition, :pop, state}` | 現在のシーンをポップ |
| `{:transition, {:push, mod, init_arg}, state}` | 新規シーンをプッシュ |
| `{:transition, {:replace, mod, init_arg}, state}` | 現在のシーンを置換 |

実際の `push_scene` / `pop_scene` / `replace_scene` は GameLoop が `Engine` 経由で呼ぶ。

---

## 4. context の構造

シーンの `update/2` に渡される context は以下のキーを持つ（`Engine.Game.context_defaults/0` とマージされる）。

| キー | 型 | 説明 |
|------|-----|------|
| `world_ref` | term() | ワールド参照。`Engine.*` の第一引数に渡す |
| `tick_ms` | pos_integer() | 1 tick のミリ秒（通常 16） |
| `now` | pos_integer() | 現在時刻（monotonic ms） |
| `elapsed` | pos_integer() | ゲーム開始からの経過 ms |
| `last_spawn_ms` | pos_integer() | 最後のスポーン時刻 |
| `weapon_levels` | map() | 武器名 → レベル |
| `frame_count` | non_neg_integer() | フレーム番号 |
| `start_ms` | pos_integer() | ゲーム開始時の monotonic ms |

---

## 5. 利用例（ヴァンサバ）

### SpawnSystem

```elixir
def maybe_spawn(world_ref, elapsed_ms, last_spawn_ms) do
  # ...
  current = Engine.get_enemy_count(world_ref)
  if current < @max_enemies do
    Engine.spawn_enemies(world_ref, kind, to_spawn)
  end
  # ...
end
```

### Playing シーン

```elixir
def update(context, state) do
  world_ref = context.world_ref

  if Engine.is_player_dead?(world_ref) do
    return_transition(:replace, Game.VampireSurvivor.Scenes.GameOver, %{}, state)
  else
    {_exp, _level, level_up_pending, _exp_to_next} = Engine.get_level_up_data(world_ref)
    if level_up_pending do
      if choices == [] do
        Engine.skip_level_up(world_ref)
        {:continue, state}
      else
        return_transition(:push, Game.VampireSurvivor.Scenes.LevelUp, %{...}, state)
      end
    end
  end
end
```

### BossAlert シーン

```elixir
def update(context, state) do
  world_ref = context.world_ref
  # ...
  Engine.spawn_boss(world_ref, boss_kind)
  {:transition, :pop, state}
end
```

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [NEXT_STEPS.md](../04_roadmap/NEXT_STEPS.md) | Step 37 の位置づけ |
| [GAME_INTERFACE.md](./GAME_INTERFACE.md) | ゲームがエンジンに提供する behaviour |
| [ELIXIR_RUST_DIVISION.md](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | Elixir/Rust 役割分担 |
| [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) | Step 44 Phoenix Channels 連携 |
