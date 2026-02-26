# Path: lib/app/nif_bridge.ex
# Summary: Rust NIF のラッパーモジュール（Rustler 経由で game_native をロード）
defmodule App.NifBridge do
  @moduledoc """
  Rust NIF のラッパーモジュール。
  `use Rustler` により、コンパイル時に `native/game_native` クレートが
  自動的にビルドされ、`.dll` がロードされる。
  """

  use Rustler,
    otp_app: :game,
    crate: :game_native

  def add(_a, _b), do: :erlang.nif_error(:nif_not_loaded)
  def create_world(), do: :erlang.nif_error(:nif_not_loaded)

  # 1.5.2: マップ障害物を設定。obstacles: [{x, y, radius, kind}, ...]
  def set_map_obstacles(_world, _obstacles), do: :erlang.nif_error(:nif_not_loaded)
  def physics_step(_world, _delta_ms), do: :erlang.nif_error(:nif_not_loaded)

  # 1.3.1: フレームイベントを取り出す（[{event_atom, arg1, arg2}] のリスト）
  def drain_frame_events(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.8: プレイヤー入力・座標取得
  def set_player_input(_world, _dx, _dy), do: :erlang.nif_error(:nif_not_loaded)
  def get_player_pos(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.9: 敵スポーン + 描画データ取得
  def spawn_enemies(_world, _kind, _count), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.10: プレイヤー HP 取得
  def get_player_hp(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.11: 弾丸数取得
  def get_bullet_count(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.12: フレーム時間・敵数取得
  def get_frame_time_ms(_world), do: :erlang.nif_error(:nif_not_loaded)
  def get_enemy_count(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.13: HUD データ一括取得（{hp, max_hp, score, elapsed_seconds}）
  def get_hud_data(_world), do: :erlang.nif_error(:nif_not_loaded)

  # Q2: 軽量フレームメタデータを1回のNIFで取得（オーバーヘッド対策）
  # 戻り値: {{hp, max_hp, score, elapsed}, {enemy_count, bullet_count, physics_ms},
  #          {exp, level, level_up_pending, exp_to_next}, {boss_alive, boss_hp, boss_max_hp}}
  def get_frame_metadata(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.14: レベルアップ関連データ取得（{exp, level, level_up_pending, exp_to_next}）
  def get_level_up_data(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.1.14/1.2.6: 武器を追加しレベルアップを確定する
  # weapon_name: "magic_wand" | "axe" | "cross" | "whip" | "fireball" | "lightning"
  def add_weapon(_world, _weapon_name), do: :erlang.nif_error(:nif_not_loaded)

  # 1.2.2: 装備中の武器スロット情報取得（[{weapon_name, level}]）
  def get_weapon_levels(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 武器選択をスキップしてレベルアップ待機を解除する（全武器MaxLv時など）
  def skip_level_up(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.2.4: 磁石エフェクト残り時間（秒）を取得
  def get_magnet_timer(_world), do: :erlang.nif_error(:nif_not_loaded)

  # 1.2.9/1.2.10: ボス・エリート敵・ゲーム状態管理

  # ボスをスポーンする（kind: :slime_king | :bat_lord | :stone_golem）
  def spawn_boss(_world, _kind), do: :erlang.nif_error(:nif_not_loaded)

  # ボスの状態を返す: {:alive, hp, max_hp} | {:none, 0.0, 0.0}
  def get_boss_info(_world), do: :erlang.nif_error(:nif_not_loaded)

  # プレイヤーが死亡しているかを返す（HP == 0 で true）
  def is_player_dead(_world), do: :erlang.nif_error(:nif_not_loaded)

  # エリート敵をスポーンする（hp_multiplier: HP 倍率）
  def spawn_elite_enemy(_world, _kind, _count, _hp_multiplier), do: :erlang.nif_error(:nif_not_loaded)

  # 1.5.1: Rust 駆動ゲームループ
  def create_game_loop_control(), do: :erlang.nif_error(:nif_not_loaded)
  def start_rust_game_loop(_world, _control, _pid), do: :erlang.nif_error(:nif_not_loaded)

  # 1.7.4: 描画スレッド起動（winit EventLoop + wgpu 骨組み）
  def start_render_thread(_world), do: :erlang.nif_error(:nif_not_loaded)
  def pause_physics(_control), do: :erlang.nif_error(:nif_not_loaded)
  def resume_physics(_control), do: :erlang.nif_error(:nif_not_loaded)

  # 1.5.3: セーブ・ロード
  def get_save_snapshot(_world), do: :erlang.nif_error(:nif_not_loaded)
  def load_save_snapshot(_world, _snapshot), do: :erlang.nif_error(:nif_not_loaded)

  # 1.5.5: デバッグ用 — ワールド状態を文字列で取得（dev ビルド時のみ有効。release では {:error, :debug_build_only}）
  def debug_dump_world(_world), do: :erlang.nif_error(:nif_not_loaded)
end
