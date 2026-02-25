# Path: lib/engine.ex
# Summary: ゲームエンジンの安定化された公開 API
defmodule Engine do
  @moduledoc """
  ゲームエンジンの安定化された公開 API（1.4.6）。

  ゲームは **Engine モジュール経由でのみ** エンジンとやり取りする。
  `App.NifBridge` や `Engine.SceneManager` を直接呼び出さず、
  このモジュールの関数を使用すること。

  ## 方針

  - **ゲーム側**: 敵スポーン、状態取得、レベルアップ制御などは `Engine` の関数を利用する
  - **シーン遷移**: `{:transition, action, state}` の戻り値で表現し、
    `Engine.SceneManager` の呼び出しはエンジン内部（GameEvents）に委譲する
  - **詳細**: `docs/06_system_design/ENGINE_API.md` を参照

  ## 利用例

      # 敵のスポーン（SpawnSystem から）
      Engine.spawn_enemies(world_ref, :slime, 5)
      Engine.spawn_elite_enemy(world_ref, :golem, 2, 3.0)

      # ボス出現（BossAlert シーンから）
      Engine.spawn_boss(world_ref, :slime_king)

      # 状態確認（Playing シーンから）
      Engine.is_player_dead?(world_ref)
      Engine.get_level_up_data(world_ref)
      Engine.skip_level_up(world_ref)

      # 敵数取得（スポーン判断用）
      Engine.get_enemy_count(world_ref)
  """

  # ── World 操作（ゲームから利用）───────────────────────────────────────

  @doc """
  通常敵をスポーンする（1.4.7: entity_registry で kind → ID に解決）。

  ## 例
      Engine.spawn_enemies(world_ref, :slime, 5)
      Engine.spawn_enemies(world_ref, :bat, 3)
      Engine.spawn_enemies(world_ref, :golem, 1)

  ## 引数
  - `world_ref` - エンジンが管理するワールド参照（context から取得）
  - `kind` - 敵種別アトム（`:slime` | `:bat` | `:golem`）
  - `count` - スポーン数
  """
  def spawn_enemies(world_ref, kind, count) do
    kind_id = resolve_enemy_id(kind)
    App.NifBridge.spawn_enemies(world_ref, kind_id, count)
  end

  @doc """
  エリート敵をスポーンする。HP 倍率を指定可能（1.4.7: entity_registry で kind → ID に解決）。

  ## 例
      Engine.spawn_elite_enemy(world_ref, :golem, 2, 3.0)
  """
  def spawn_elite_enemy(world_ref, kind, count, hp_multiplier) do
    kind_id = resolve_enemy_id(kind)
    App.NifBridge.spawn_elite_enemy(world_ref, kind_id, count, hp_multiplier)
  end

  @doc """
  ボスをスポーンする（1.4.7: entity_registry で kind → ID に解決）。

  ## 例
      Engine.spawn_boss(world_ref, :slime_king)
      Engine.spawn_boss(world_ref, :bat_lord)
      Engine.spawn_boss(world_ref, :stone_golem)
  """
  def spawn_boss(world_ref, kind) do
    kind_id = resolve_boss_id(kind)
    App.NifBridge.spawn_boss(world_ref, kind_id)
  end

  @doc """
  現在の敵数を返す。スポーン判断に利用。

  ## 例
      current = Engine.get_enemy_count(world_ref)
      if current < max_enemies do
        Engine.spawn_enemies(world_ref, kind, to_spawn)
      end
  """
  def get_enemy_count(world_ref) do
    App.NifBridge.get_enemy_count(world_ref)
  end

  @doc """
  プレイヤーが死亡しているかどうかを返す。

  ## 例
      if Engine.is_player_dead?(world_ref) do
        # GameOver への遷移
      end
  """
  def is_player_dead?(world_ref) do
    App.NifBridge.is_player_dead(world_ref)
  end

  @doc """
  レベルアップ関連データを取得する。

  ## 戻り値
  `{exp, level, level_up_pending, exp_to_next}`

  ## 例
      {exp, level, level_up_pending, exp_to_next} = Engine.get_level_up_data(world_ref)
      if level_up_pending do
        # LevelUp シーンへ遷移
      end
  """
  def get_level_up_data(world_ref) do
    App.NifBridge.get_level_up_data(world_ref)
  end

  @doc """
  武器選択をスキップしてレベルアップ待機を解除する。
  全武器 MaxLv のときなどに使用。

  ## 例
      if choices == [] do
        Engine.skip_level_up(world_ref)
      end
  """
  def skip_level_up(world_ref) do
    App.NifBridge.skip_level_up(world_ref)
  end

  # ── エンジン内部用（GameEvents が使用。ゲームは通常呼ばない）─────────────
  # 以下は安定 API の一部として文書化。新規ゲームでは GameEvents が利用するため、
  # ゲームコードから直接呼ぶ必要はない。

  @doc """
  ワールドを生成する。GameEvents の init で呼ばれる。
  ゲームから直接呼ぶことはない。
  """
  def create_world do
    App.NifBridge.create_world()
  end

  @doc """
  1.5.2: マップ障害物を設定する。
  GameEvents の init で呼ばれる。obstacles は MapLoader.obstacles_for_map/1 の戻り値。
  """
  def set_map_obstacles(world_ref, obstacles) do
    App.NifBridge.set_map_obstacles(world_ref, obstacles)
  end

  @doc """
  1.5.1: ゲームループ制御用リソースを作成する。
  pause_physics / resume_physics で使用。
  """
  def create_game_loop_control do
    App.NifBridge.create_game_loop_control()
  end

  @doc """
  1.5.1: Rust 駆動の高精度ゲームループを起動する。
  pid には GameEvents の self() を渡す。
  """
  def start_rust_game_loop(world_ref, control_ref, pid) do
    App.NifBridge.start_rust_game_loop(world_ref, control_ref, pid)
  end

  @doc """
  1.5.1: LevelUp・BossAlert 中に physics を一時停止する。
  """
  def pause_physics(control_ref) do
    App.NifBridge.pause_physics(control_ref)
  end

  @doc """
  1.5.1: physics を再開する。
  """
  def resume_physics(control_ref) do
    App.NifBridge.resume_physics(control_ref)
  end

  @doc """
  物理演算を1ステップ実行する。GameEvents の tick から呼ばれる。
  ゲームから直接呼ぶことはない。
  """
  def physics_step(world_ref, delta_ms) do
    App.NifBridge.physics_step(world_ref, delta_ms)
  end

  @doc """
  プレイヤー入力を設定する。GameEvents が InputHandler の結果を渡す。
  ゲームから直接呼ぶことはない。
  """
  def set_player_input(world_ref, dx, dy) do
    App.NifBridge.set_player_input(world_ref, dx, dy)
  end

  @doc """
  フレームイベントをドレインする。GameEvents が EventBus にブロードキャストする。
  ゲームから直接呼ぶことはない。
  """
  def drain_frame_events(world_ref) do
    App.NifBridge.drain_frame_events(world_ref)
  end

  @doc """
  フレームメタデータを取得する。FrameCache や HUD 描画に利用。
  GameEvents が使用。ゲームから直接呼ぶことはない。
  """
  def get_frame_metadata(world_ref) do
    App.NifBridge.get_frame_metadata(world_ref)
  end

  @doc """
  武器を追加する（1.4.7: entity_registry で weapon → ID に解決）。
  GameEvents の weapon 選択処理で呼ばれる。

  ## 引数
  - `weapon_name` - 追加する武器の名前（atom または string）
  """
  def add_weapon(world_ref, weapon_name) when is_binary(weapon_name) do
    weapon_id = resolve_weapon_id(String.to_atom(weapon_name))
    App.NifBridge.add_weapon(world_ref, weapon_id)
  end

  def add_weapon(world_ref, weapon) when is_atom(weapon) do
    weapon_id = resolve_weapon_id(weapon)
    App.NifBridge.add_weapon(world_ref, weapon_id)
  end

  @doc """
  装備中の武器スロット情報を取得する。GameEvents が context 構築に使用。
  ゲームから直接呼ぶことはない。
  """
  def get_weapon_levels(world_ref) do
    App.NifBridge.get_weapon_levels(world_ref)
  end

  # ── 1.5.3: セーブ・ロード ─────────────────────────────────────────────

  @doc """
  現在のゲーム状態をセーブする。saves/session.dat に保存。

  ## 例
      Engine.save_session(world_ref)
  """
  def save_session(world_ref), do: Engine.SaveManager.save_session(world_ref)

  @doc """
  セーブデータをロードして world_ref に復元する。

  ## 戻り値
  - `:ok` - 復元成功
  - `:no_save` - セーブファイルなし
  - `{:error, reason}` - ロード失敗
  """
  def load_session(world_ref), do: Engine.SaveManager.load_session(world_ref)

  @doc """
  セーブファイルが存在するかどうか。
  """
  def has_save?, do: Engine.SaveManager.has_save?()

  @doc """
  ハイスコアを記録する。
  """
  def save_high_score(score), do: Engine.SaveManager.save_high_score(score)

  @doc """
  保存されているハイスコア一覧を取得する。
  """
  def load_high_scores, do: Engine.SaveManager.load_high_scores()

  @doc """
  ベストスコア（1位）を取得する。
  """
  def best_score, do: Engine.SaveManager.best_score()

  # ── 1.5.4: マルチプレイ・ルーム管理 ────────────────────────────────────

  @doc """
  新規ルームを起動する。各ルームは独立した GameEvents + GameWorld を持つ。

  ## 戻り値
  - `{:ok, pid}` - 起動成功
  - `{:error, :already_started}` - 同じ room_id のルームが既に存在

  ## 例
      Engine.start_room("room_123")
  """
  def start_room(room_id), do: Engine.RoomSupervisor.start_room(room_id)

  @doc """
  ルームを終了する。GameWorld が解放される。

  ## 戻り値
  - `:ok` - 終了成功
  - `{:error, :not_found}` - ルームが存在しない
  """
  def stop_room(room_id), do: Engine.RoomSupervisor.stop_room(room_id)

  @doc """
  アクティブなルーム ID のリストを返す。
  """
  def list_rooms, do: Engine.RoomSupervisor.list_rooms()

  @doc """
  ルーム ID に対応する GameEvents の pid を返す。
  メインルームは `:main`、Phoenix Channel 連携時はルーム文字列を使用。

  ## 例
      {:ok, pid} = Engine.get_loop_for_room(:main)
  """
  def get_loop_for_room(room_id), do: Engine.RoomRegistry.get_loop(room_id)

  # ── シーン操作（GameEvents が transition 処理で使用）───────────────────────
  # ゲームは {:transition, {:push, mod, arg}, state} 等の戻り値で意図を伝え、
  # 実際の push_scene 等の呼び出しは GameEvents が行う。

  @doc """
  新規シーンをスタックにプッシュする。GameEvents の transition 処理で使用。
  ゲームは update の戻り値で `{:transition, {:push, mod, arg}, state}` を返す。
  """
  defdelegate push_scene(module, init_arg \\ %{}), to: Engine.SceneManager

  @doc """
  現在のシーンをポップする。GameEvents の transition 処理で使用。
  ゲームは `{:transition, :pop, state}` を返す。
  """
  defdelegate pop_scene(), to: Engine.SceneManager

  @doc """
  現在のシーンを別シーンに置換する。GameOver 遷移・リスタート時に使用。
  ゲームは `{:transition, {:replace, mod, arg}, state}` を返す。
  """
  defdelegate replace_scene(module, init_arg \\ %{}), to: Engine.SceneManager

  @doc "現在のシーンを返す。GameEvents が使用。"
  defdelegate current_scene(), to: Engine.SceneManager, as: :current

  @doc "描画用の現在シーン種別を返す。FrameCache 等が使用。"
  defdelegate render_type(), to: Engine.SceneManager

  @doc "現在シーンの state を更新する。GameEvents が使用。"
  defdelegate update_current_scene(fun), to: Engine.SceneManager, as: :update_current

  # ── 1.4.7: entity_registry による ID 解決（内部用）─────────────────────

  defp resolve_enemy_id(kind) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    Map.fetch!(game.entity_registry().enemies, kind)
  end

  defp resolve_boss_id(kind) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    Map.fetch!(game.entity_registry().bosses, kind)
  end

  defp resolve_weapon_id(weapon) do
    game = Application.get_env(:game, :current, Game.VampireSurvivor)
    Map.fetch!(game.entity_registry().weapons, weapon)
  end
end
