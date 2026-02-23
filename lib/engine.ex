defmodule Engine do
  @moduledoc """
  ゲームエンジンの安定化された公開 API（Step 37）。

  ゲームは **Engine モジュール経由でのみ** エンジンとやり取りする。
  `Game.NifBridge` や `Engine.SceneManager` を直接呼び出さず、
  このモジュールの関数を使用すること。

  ## 方針

  - **ゲーム側**: 敵スポーン、状態取得、レベルアップ制御などは `Engine` の関数を利用する
  - **シーン遷移**: `{:transition, action, state}` の戻り値で表現し、
    `Engine.SceneManager` の呼び出しはエンジン内部（GameLoop）に委譲する
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
  通常敵をスポーンする。

  ## 例
      Engine.spawn_enemies(world_ref, :slime, 5)
      Engine.spawn_enemies(world_ref, :bat, 3)
      Engine.spawn_enemies(world_ref, :golem, 1)

  ## 引数
  - `world_ref` - エンジンが管理するワールド参照（context から取得）
  - `kind` - 敵種別（`:slime` | `:bat` | `:golem`）
  - `count` - スポーン数
  """
  def spawn_enemies(world_ref, kind, count) do
    Game.NifBridge.spawn_enemies(world_ref, kind, count)
  end

  @doc """
  エリート敵をスポーンする。HP 倍率を指定可能。

  ## 例
      Engine.spawn_elite_enemy(world_ref, :golem, 2, 3.0)
  """
  def spawn_elite_enemy(world_ref, kind, count, hp_multiplier) do
    Game.NifBridge.spawn_elite_enemy(world_ref, kind, count, hp_multiplier)
  end

  @doc """
  ボスをスポーンする。

  ## 例
      Engine.spawn_boss(world_ref, :slime_king)
      Engine.spawn_boss(world_ref, :bat_lord)
      Engine.spawn_boss(world_ref, :stone_golem)
  """
  def spawn_boss(world_ref, kind) do
    Game.NifBridge.spawn_boss(world_ref, kind)
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
    Game.NifBridge.get_enemy_count(world_ref)
  end

  @doc """
  プレイヤーが死亡しているかどうかを返す。

  ## 例
      if Engine.is_player_dead?(world_ref) do
        # GameOver への遷移
      end
  """
  def is_player_dead?(world_ref) do
    Game.NifBridge.is_player_dead(world_ref)
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
    Game.NifBridge.get_level_up_data(world_ref)
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
    Game.NifBridge.skip_level_up(world_ref)
  end

  # ── エンジン内部用（GameLoop が使用。ゲームは通常呼ばない）─────────────
  # 以下は安定 API の一部として文書化。新規ゲームでは GameLoop が利用するため、
  # ゲームコードから直接呼ぶ必要はない。

  @doc """
  ワールドを生成する。GameLoop の init で呼ばれる。
  ゲームから直接呼ぶことはない。
  """
  def create_world do
    Game.NifBridge.create_world()
  end

  @doc """
  物理演算を1ステップ実行する。GameLoop の tick から呼ばれる。
  ゲームから直接呼ぶことはない。
  """
  def physics_step(world_ref, delta_ms) do
    Game.NifBridge.physics_step(world_ref, delta_ms)
  end

  @doc """
  プレイヤー入力を設定する。GameLoop が InputHandler の結果を渡す。
  ゲームから直接呼ぶことはない。
  """
  def set_player_input(world_ref, dx, dy) do
    Game.NifBridge.set_player_input(world_ref, dx, dy)
  end

  @doc """
  フレームイベントをドレインする。GameLoop が EventBus にブロードキャストする。
  ゲームから直接呼ぶことはない。
  """
  def drain_frame_events(world_ref) do
    Game.NifBridge.drain_frame_events(world_ref)
  end

  @doc """
  フレームメタデータを取得する。FrameCache や HUD 描画に利用。
  GameLoop が使用。ゲームから直接呼ぶことはない。
  """
  def get_frame_metadata(world_ref) do
    Game.NifBridge.get_frame_metadata(world_ref)
  end

  @doc """
  武器を追加する。GameLoop の weapon 選択処理で呼ばれる。
  ゲームは LevelUp シーンの戻り値 `{:transition, :pop, state}` 等で間接的にトリガーする。
  """
  def add_weapon(world_ref, weapon_name) when is_binary(weapon_name) do
    Game.NifBridge.add_weapon(world_ref, weapon_name)
  end

  def add_weapon(world_ref, weapon) when is_atom(weapon) do
    Game.NifBridge.add_weapon(world_ref, to_string(weapon))
  end

  @doc """
  装備中の武器スロット情報を取得する。GameLoop が context 構築に使用。
  ゲームから直接呼ぶことはない。
  """
  def get_weapon_levels(world_ref) do
    Game.NifBridge.get_weapon_levels(world_ref)
  end

  # ── シーン操作（GameLoop が transition 処理で使用）───────────────────────
  # ゲームは {:transition, {:push, mod, arg}, state} 等の戻り値で意図を伝え、
  # 実際の push_scene 等の呼び出しは GameLoop が行う。

  @doc """
  新規シーンをスタックにプッシュする。GameLoop の transition 処理で使用。
  ゲームは update の戻り値で `{:transition, {:push, mod, arg}, state}` を返す。
  """
  defdelegate push_scene(module, init_arg \\ %{}), to: Engine.SceneManager

  @doc """
  現在のシーンをポップする。GameLoop の transition 処理で使用。
  ゲームは `{:transition, :pop, state}` を返す。
  """
  defdelegate pop_scene(), to: Engine.SceneManager

  @doc """
  現在のシーンを別シーンに置換する。GameOver 遷移・リスタート時に使用。
  ゲームは `{:transition, {:replace, mod, arg}, state}` を返す。
  """
  defdelegate replace_scene(module, init_arg \\ %{}), to: Engine.SceneManager

  @doc "現在のシーンを返す。GameLoop が使用。"
  defdelegate current_scene(), to: Engine.SceneManager, as: :current

  @doc "描画用の現在シーン種別を返す。FrameCache 等が使用。"
  defdelegate render_type(), to: Engine.SceneManager

  @doc "現在シーンの state を更新する。GameLoop が使用。"
  defdelegate update_current_scene(fun), to: Engine.SceneManager, as: :update_current
end
