# ゲーム仕様書: Survivor（ヴァンパイアサバイバーライク）

**プロジェクト名**: Elixir x Rust Survivor  
**ジャンル**: ヴァンパイアサバイバーライク（ローグライト・バレットヘル）  
**プラットフォーム**: Windows / macOS / Linux（wgpu 対応環境）

エンジン仕様は [SPEC_ENGINE.md](SPEC_ENGINE.md) を参照。本ドキュメントは Survivor 用のゲームデザイン・ECS コンポーネント・敵 AI・スポーン・アトラスを定義する。

---

## 目次

1. [ゲームデザイン仕様](#1-ゲームデザイン仕様)
2. [Survivor 用 ECS コンポーネント](#2-survivor-用-ecs-コンポーネント)
3. [敵 AI・移動](#3-敵-ai移動)
4. [SpawnSystem](#4-spawnsystem)
5. [テクスチャアトラス（Survivor）](#5-テクスチャアトラスsurvivor)
6. [レンダリングデータデコード例](#6-レンダリングデータデコード例)

---

## 1. ゲームデザイン仕様

### 1.1 ゲーム概要

プレイヤーは無限に湧き続ける敵の群れを自動攻撃で生き延びる。時間経過とともに敵の数・強さが増加し、プレイヤーは経験値を集めてキャラクターを強化する。

### 1.2 プレイヤー仕様

| パラメータ | 初期値 | 最大値 | 備考 |
|---|---|---|---|
| HP | 100 | 500（強化後） | 0 で即死 |
| 移動速度 | 150 px/s | 400 px/s | 8方向移動 |
| 攻撃力 | 10 | 200 | 武器依存 |
| 経験値倍率 | 1.0x | 3.0x | レベルアップで上昇 |
| 無敵時間（被弾後） | 0.5 秒 | — | 連続ダメージ防止 |

#### プレイヤー入力

| 入力 | アクション |
|---|---|
| WASD / 矢印キー | 移動 |
| ESC | ポーズ |
| Enter（レベルアップ時） | 武器選択確定 |

### 1.3 敵仕様

#### 敵タイプ一覧

| タイプ | HP | 速度 | 攻撃力 | 出現開始時間 | 特徴 |
|---|---|---|---|---|---|
| Slime | 20 | 80 px/s | 5 | 0秒〜 | 最基本。直進してくる |
| Bat | 15 | 140 px/s | 8 | 30秒〜 | 高速。ジグザグ移動 |
| Skeleton | 60 | 60 px/s | 15 | 60秒〜 | 高HP。集団行動 |
| Ghost | 40 | 100 px/s | 12 | 120秒〜 | 壁すり抜け（障害物無視） |
| Golem | 300 | 30 px/s | 40 | 180秒〜 | ボス級。範囲攻撃 |

#### 敵スポーンルール

```
スポーン数 = base_count × (1 + elapsed_minutes × 0.5)
スポーン間隔 = max(0.5秒, 3.0秒 - elapsed_minutes × 0.2)
スポーン位置 = プレイヤーから 800〜1200px の円周上（画面外）
```

### 1.4 武器システム

プレイヤーはレベルアップ時に 3 択から武器・強化を選ぶ。

| 武器 | 攻撃方式 | 初期ダメージ | 発射間隔 |
|---|---|---|---|
| Magic Wand | 最近接敵へ自動発射 | 10 | 0.8秒 |
| Garlic | 自身周囲に常時ダメージ | 5/秒 | — |
| Axe | 放物線投擲（範囲） | 20 | 1.5秒 |
| Lightning | 連鎖電撃（最大5体） | 15 | 1.2秒 |
| Whip | 左右交互に横一直線 | 25 | 1.0秒 |

### 1.5 ステージ仕様

- **マップサイズ**: 4096 × 4096 px（スクロールあり）
- **カメラ**: プレイヤー中心追従（画面サイズ: 1280 × 720）
- **障害物**: 木・岩（Ghost 以外は回避）
- **ゲーム時間**: 最大 30 分（クリア条件）

---

## 2. Survivor 用 ECS コンポーネント

### 2.1 EnemyWorld（SoA）

```rust
// native/game_native/src/ecs/world.rs

pub struct EnemyWorld {
    // --- 毎フレーム更新（位置・速度） ---
    pub positions_x: Vec<f32>,       // ワールド X 座標
    pub positions_y: Vec<f32>,       // ワールド Y 座標
    pub velocities_x: Vec<f32>,      // X 方向速度 (px/s)
    pub velocities_y: Vec<f32>,      // Y 方向速度 (px/s)

    // --- 低頻度更新（HP・状態） ---
    pub health: Vec<i32>,            // 現在 HP
    pub max_health: Vec<i32>,        // 最大 HP
    pub damage: Vec<i32>,            // 攻撃力

    // --- ほぼ不変（スプライト・タイプ） ---
    pub sprite_ids: Vec<u16>,        // テクスチャアトラス内インデックス
    pub enemy_types: Vec<EnemyType>, // 敵タイプ（AI 挙動分岐用）
    pub speeds: Vec<f32>,            // 基本移動速度

    // --- フラグ ---
    pub alive: Vec<bool>,            // 生存フラグ
    pub count: usize,                // 有効エンティティ数
    pub capacity: usize,             // 確保済みキャパシティ
}
```

### 2.2 プレイヤーコンポーネント

```rust
pub struct PlayerState {
    pub position_x: f32,
    pub position_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub health: i32,
    pub max_health: i32,
    pub level: u32,
    pub experience: u32,
    pub invincible_timer: f32,  // 無敵時間残り（秒）
    pub weapons: Vec<WeaponState>,
}
```

### 2.3 弾丸コンポーネント

```rust
pub struct BulletWorld {
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage: Vec<i32>,
    pub lifetime: Vec<f32>,    // 残り寿命（秒）
    pub sprite_ids: Vec<u16>,
    pub alive: Vec<bool>,
    pub count: usize,
}
```

---

## 3. 敵 AI・移動

### 3.1 AI ステートマシン

```rust
// native/game_native/src/physics/movement.rs

#[derive(Clone, Copy)]
pub enum AiState {
    Chase,          // プレイヤーに直進
    Zigzag { phase: f32 },  // ジグザグ移動（Bat）
    Flock,          // 集団行動（Skeleton）
    Wander,         // 徘徊（Ghost）
}

pub fn update_enemy_movement(world: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    for i in 0..world.count {
        if !world.alive[i] { continue; }

        let dx = player_x - world.positions_x[i];
        let dy = player_y - world.positions_y[i];
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);

        match world.ai_states[i] {
            AiState::Chase => {
                world.velocities_x[i] = (dx / dist) * world.speeds[i];
                world.velocities_y[i] = (dy / dist) * world.speeds[i];
            }
            AiState::Zigzag { ref mut phase } => {
                *phase += dt * 3.0;
                let perp_x = -dy / dist;
                let perp_y =  dx / dist;
                world.velocities_x[i] = (dx / dist + perp_x * phase.sin()) * world.speeds[i];
                world.velocities_y[i] = (dy / dist + perp_y * phase.sin()) * world.speeds[i];
            }
            // ... 他の AI タイプ
        }

        world.positions_x[i] += world.velocities_x[i] * dt;
        world.positions_y[i] += world.velocities_y[i] * dt;
    }
}
```

---

## 4. SpawnSystem

```elixir
# lib/game/systems/spawn_system.ex
defmodule Game.SpawnSystem do
  @spawn_table [
    # {開始秒, 敵タイプ, 基本スポーン数}
    {0,   :slime,    5},
    {30,  :bat,      3},
    {60,  :skeleton, 2},
    {120, :ghost,    2},
    {180, :golem,    1},
  ]

  def maybe_spawn(world_ref, elapsed_seconds) do
    @spawn_table
    |> Enum.filter(fn {start, _type, _count} -> elapsed_seconds >= start end)
    |> Enum.each(fn {_start, type, base_count} ->
      count = trunc(base_count * (1 + elapsed_seconds / 60.0 * 0.5))
      Game.NifBridge.spawn_enemies(world_ref, type, count)
    end)
  end
end
```

---

## 5. テクスチャアトラス（Survivor）

```
atlas.png: 2048 × 2048 px
├── Slime:    [0, 0] 〜 [64, 64]（アニメーション 4 フレーム × 横並び）
├── Bat:      [256, 0] 〜 [320, 64]
├── Skeleton: [512, 0] 〜 [576, 64]
├── Ghost:    [768, 0] 〜 [832, 64]
├── Golem:    [0, 64] 〜 [128, 192]（128×128 px）
├── Player:   [0, 256] 〜 [64, 320]
└── Bullets:  [0, 512] 〜 [512, 544]（各種弾丸）
```

---

## 6. レンダリングデータデコード例

同一バイナリ形式を Elixir で扱う場合のデコード例（デバッグ・リプレイ等）。

```elixir
# 同一バイナリ形式を Elixir で扱う場合のデコード例（デバッグ・リプレイ等）
defmodule Game.RenderDataDecoder do
  def decode(<<
    entity_count::uint32-little,
    frame_id::uint32-little,
    px::float32-little, py::float32-little,
    php::int32-little, pmax_hp::int32-little, plevel::uint32-little,
    rest::binary
  >>) do
    {enemies, rest2} = decode_enemies(rest, entity_count, [])
    %{
      frame_id: frame_id,
      player: %{x: px, y: py, hp: php, max_hp: pmax_hp, level: plevel},
      enemies: enemies
    }
  end

  defp decode_enemies(bin, 0, acc), do: {Enum.reverse(acc), bin}
  defp decode_enemies(
    <<x::float32-little, y::float32-little, sprite::uint16-little, rest::binary>>,
    n, acc
  ) do
    decode_enemies(rest, n - 1, [{x, y, sprite} | acc])
  end
end
```
