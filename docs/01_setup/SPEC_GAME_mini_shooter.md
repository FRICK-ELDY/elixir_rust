# ゲーム仕様書: Mini Shooter

**プロジェクト名**: Elixir x Rust Mini Shooter  
**ジャンル**: トップダウン・シューティング（ミニゲーム）  
**プラットフォーム**: Windows / macOS / Linux（wgpu 対応環境）

エンジン仕様は [SPEC_ENGINE.md](SPEC_ENGINE.md) を参照。本ドキュメントは Mini Shooter 用のゲームデザイン・ECS コンポーネント・敵挙動・スポーン・アトラスを定義する。

---

## 目次

1. [ゲームデザイン仕様](#1-ゲームデザイン仕様)
2. [Mini Shooter 用 ECS コンポーネント](#2-mini-shooter-用-ecs-コンポーネント)
3. [敵 AI・移動](#3-敵-ai移動)
4. [SpawnSystem・ウェーブ](#4-spawnsystemウェーブ)
5. [テクスチャアトラス（Mini Shooter）](#5-テクスチャアトラスmini-shooter)

---

## 1. ゲームデザイン仕様

### 1.1 ゲーム概要

画面上部から出現する敵を自機の弾で撃ち落とすトップダウンシューティング。ウェーブ制で敵数・種類が増加し、スコアと残機でクリア条件を競う。Survivor より小規模（敵数・武器数は少なめ）で、同じエンジン上で動作するミニゲームとして位置づける。

### 1.2 プレイヤー仕様

| パラメータ | 初期値 | 最大値 | 備考 |
|---|---|---|---|
| HP（残機） | 3 | 5（パワーアップ時） | 0 でゲームオーバー |
| 移動速度 | 200 px/s | 280 px/s | 左右＋前後移動 |
| 弾ダメージ | 10 | 30 | パワーアップで上昇 |
| 発射間隔 | 0.15 秒 | 0.08 秒 | 連射速度 |
| 無敵時間（被弾後） | 1.5 秒 | — | スポーン時も適用 |

#### プレイヤー入力

| 入力 | アクション |
|---|---|
| WASD / 矢印キー | 8方向移動 |
| スペース / Z | ショット |
| ESC | ポーズ |

### 1.3 敵仕様

#### 敵タイプ一覧

| タイプ | HP | 速度 | 攻撃 | 出現ウェーブ | 特徴 |
|---|---|---|---|---|---|
| Drone | 15 | 60 px/s | なし | 1〜 | 直下に降下。撃破でスコア 100 |
| Tank | 50 | 20 px/s | なし | 2〜 | ゆっくり降下。撃破でスコア 300 |
| Shooter | 25 | 40 px/s | 直線弾 | 3〜 | 一定間隔でプレイヤー方向に弾発射。撃破でスコア 200 |
| Boss | 200 | 15 px/s | 3-way 弾 | 5 の倍数ウェーブ | 画面中央付近で往復。撃破でスコア 1000 |

#### 敵スポーンルール

```
ウェーブ N 開始時:
  そのウェーブで出現する敵タイプをテーブルから取得
  スポーン数 = wave_base_count(N)（ウェーブごとに定義）
  スポーン位置 = 画面上端 Y=0 付近、X はランダム（マージン内）
  スポーン間隔 = 0.8 秒 × (1 - N × 0.02)、最小 0.3 秒

ウェーブクリア条件: そのウェーブの全敵撃破
```

### 1.4 武器・弾仕様

- **通常弾**: プレイヤー正面に 1 発。直進。ダメージはプレイヤーの「弾ダメージ」。
- **パワーアップ**: ウェーブクリアまたは特定敵撃破でドロップ（オプション）。取得で「弾ダメージ＋5」または「発射間隔短縮」のいずれか。
- 敵弾（Shooter / Boss）: プレイヤーまたは直下方向に発射。当たると残機－1。

### 1.5 ステージ仕様

- **マップサイズ**: 固定 1280 × 720 px（スクロールなし）
- **プレイヤー初期位置**: 画面中央下 (640, 600)
- **敵出現域**: 画面上部 Y < 120
- **ゲーム終了**: 残機 0 でゲームオーバー。任意の最大ウェーブでクリア扱い（例: 10 ウェーブ）。

---

## 2. Mini Shooter 用 ECS コンポーネント

### 2.1 EnemyWorld（SoA）

Survivor と同様の SoA だが、敵タイプは Mini Shooter 用 enum。

```rust
pub enum MiniShooterEnemyType {
    Drone,
    Tank,
    Shooter,
    Boss,
}

pub struct EnemyWorld {
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub health: Vec<i32>,
    pub max_health: Vec<i32>,
    pub sprite_ids: Vec<u16>,
    pub enemy_types: Vec<MiniShooterEnemyType>,
    pub speeds: Vec<f32>,
    pub shoot_timers: Vec<f32>,  // Shooter/Boss 用。0 で発射してリセット
    pub alive: Vec<bool>,
    pub count: usize,
    pub capacity: usize,
}
```

### 2.2 プレイヤーコンポーネント

```rust
pub struct PlayerState {
    pub position_x: f32,
    pub position_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub lives: i32,
    pub invincible_timer: f32,
    pub shot_cooldown: f32,   // 発射間隔カウント
    pub damage: i32,
    pub fire_interval: f32,
    pub score: u32,
}
```

### 2.3 弾丸コンポーネント（自機弾・敵弾を同一または別 SoA で管理）

```rust
pub struct BulletWorld {
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage: Vec<i32>,
    pub is_player_bullet: Vec<bool>,  // 自機弾 true / 敵弾 false
    pub lifetime: Vec<f32>,
    pub sprite_ids: Vec<u16>,
    pub alive: Vec<bool>,
    pub count: usize,
}
```

---

## 3. 敵 AI・移動

### 3.1 挙動概要

| タイプ | 移動 | 攻撃 |
|---|---|---|
| Drone | 真下に等速 | なし |
| Tank | 真下に低速 | なし |
| Shooter | 真下＋左右ゆらぎ | 2 秒間隔でプレイヤー方向に 1 発 |
| Boss | 画面中央付近で左右往復 | 3 秒間隔で 3-way 弾 |

### 3.2 実装方針

- `update_enemy_movement`: プレイヤー位置を参照するのは Shooter の弾方向計算のみ。それ以外は時間ベースまたは固定速度。
- 敵弾生成: Shooter / Boss の `shoot_timer` が 0 を下回ったら弾を 1 発（または 3 発）追加し、タイマーをリセット。
- 画面外（Y > 720 または Y < -50）で敵を削除し、ウェーブ残数から減算（ミス扱いにするかはデザイン次第。本仕様では敵が画面下端を抜けても残機は減らさず、ウェーブ残数のみ減らす）。

---

## 4. SpawnSystem・ウェーブ

```elixir
# lib/game/systems/spawn_system_mini_shooter.ex
defmodule Game.SpawnSystem.MiniShooter do
  @wave_table [
    # ウェーブ番号 1-based, 敵タイプ, そのウェーブでの出現数
    {1, [:drone], 8},
    {2, [:drone, :tank], 12},
    {3, [:drone, :tank, :shooter], 15},
    {4, [:drone, :tank, :shooter], 18},
    {5, [:drone, :tank, :shooter, :boss], 20},
    # 6〜 は 5 のパターンを繰り返しつつ数だけ増加
  ]

  def wave_enemies(wave_num) do
    entry = Enum.find(@wave_table, fn {n, _types, _count} -> n == wave_num end)
    if entry, do: elem(entry, 1), else: [:drone, :tank, :shooter]
  end

  def wave_count(wave_num) do
    entry = Enum.find(@wave_table, fn {n, _types, _count} -> n == wave_num end)
    if entry, do: elem(entry, 2), else: 10 + wave_num * 2
  end

  def spawn_for_wave(world_ref, wave_num, elapsed_in_wave) do
    types = wave_enemies(wave_num)
    total = wave_count(wave_num)
    # 経過時間と間隔から今回スポーンする数を決定し、NIF で spawn_enemies を呼ぶ
    # 例: spawn_enemies(world_ref, :drone, n_drone), spawn_enemies(world_ref, :tank, n_tank), ...
  end
end
```

---

## 5. テクスチャアトラス（Mini Shooter）

```
atlas_mini_shooter.png: 1024 × 1024 px（Survivor より小規模で可）
├── Player:    [0, 0] 〜 [48, 48]
├── Drone:     [64, 0] 〜 [112, 48]
├── Tank:      [128, 0] 〜 [176, 48]
├── Shooter:   [192, 0] 〜 [240, 48]
├── Boss:      [0, 64] 〜 [96, 160]   （大きめ）
├── PlayerBullet: [256, 0] 〜 [272, 16]
└── EnemyBullet:  [288, 0] 〜 [304, 16]
```

---

## 6. Survivor との差分まとめ

| 項目 | Survivor | Mini Shooter |
|---|---|---|
| 敵数規模 | 数千体想定 | 数十体程度 |
| プレイヤー攻撃 | 自動攻撃・複数武器 | 手動ショット・単一弾種（パワーアップで強化） |
| 進行 | 時間経過で難易度上昇 | ウェーブクリアで進行 |
| マップ | 4096×4096 スクロール | 1280×720 固定 |
| 敵 AI | Chase / Zigzag / Flock 等 | 直下降下・往復・一定間隔射撃 |

同一エンジン（SPEC_ENGINE）の NIF・wgpu・物理・ECS パターンを流用し、上記のゲーム固有データとロジックのみ Mini Shooter 用に実装する。
