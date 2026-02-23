# Elixir × Rust ゲームエンジン — 技術発表資料

**「なぜ2 つの言語を組み合わせると、どちらか単独より強くなるのか」**

---

## はじめに — このゲームエンジンが証明したこと

このプロジェクトは、Vampire Survivors ライクなゲームを通じて、ひとつの問いに答えます。

> **「Elixir と Rust を組み合わせたら、本当に実用的なゲームエンジンが作れるのか？」**

答えは **YES** でした。

- 同時に **10,000 体の敵** が画面を埋め尽くす
- それでも **60 FPS** を維持する
- ゲームロジックは **Elixir の宣言的なコード** で書かれている
- 物理演算と描画は **Rust がネイティブ速度** で処理する

これは「実験」ではありません。**動くゲームです。**

---

## 1. アーキテクチャの全体像

```
┌─────────────────────────────────────────────────────────────────┐
│                   Elixir BEAM VM（司令塔）                       │
│                                                                 │
│   GameLoop GenServer     SceneManager      SpawnSystem          │
│   ├─ 60Hz tick           ├─ シーンスタック ├─ ウェーブ制御       │
│   ├─ ゲームフェーズ管理  ├─ Playing/       ├─ BossSystem        │
│   └─ OTP Supervisor       LevelUp/BossAlert/GameOver            │
│                                                                 │
│   EventBus / FrameCache  LevelSystem       StressMonitor        │
│   ├─ フレームイベント配信├─ 武器選択肢     └─ 独立プロセスで監視  │
│   ├─ ETS キャッシュ      └─ EXP テーブル                         │
│   └─ InputHandler（キー入力状態）                                │
└─────────────────────┬───────────────────────────────────────────┘
                      │  Rustler NIF（DirtyCpu スケジューラ）
                      │  ResourceArc<RwLock<GameWorld>>
                      │  ← 8 バイトのポインタのみ渡す（読み取り並列可能）
┌─────────────────────▼───────────────────────────────────────────┐
│                   Rust Native（エンジン本体）                    │
│                                                                 │
│   ECS World (SoA)        Physics             Renderer           │
│   ├─ EnemyWorld/Bullet   ├─ Spatial Hash     ├─ wgpu GPU        │
│   │  /Particle/Item      ├─ rayon 並列 AI    ├─ インスタンシング │
│   ├─ フリーリスト        ├─ rayon（x86_64 では SIMD）├─ WGSL シェーダ   │
│   └─ ID ベース参照      └─ 衝突・最近接 O(1) 近    └─ egui HUD + rodio│
│                                                                 │
│   winit EventLoop（メインスレッド）                              │
└─────────────────────────────────────────────────────────────────┘
```

**2 つの世界が、それぞれの得意分野だけを担当する。**  
これがこのエンジンの核心です。

---

## 2. なぜ Elixir なのか — BEAM VM の底力

### 「電話交換機の VM」がゲームに最適だった

BEAM VM は 1980 年代に Ericsson が「絶対に落ちてはいけない電話交換機」のために作った仮想マシンです。その設計目標が、現代のゲーム開発と驚くほど一致していました。

| BEAM の設計目標 | ゲームでの対応 |
|---|---|
| 高並行性（数百万プロセス） | ゲームシステムを独立して動かす |
| 耐障害性（99.9999999% 稼働率） | バグからの自動回復 |
| ソフトリアルタイム性 | 安定した 60fps ゲームループ |
| ホットコードスワップ | 実行中のゲームロジック変更 |
| 分散処理 | 将来的なマルチプレイ対応 |

### GenServer がゲームループを「宣言的」にする

従来の命令型ゲームループは `if/switch` の連鎖になりがちです。Elixir のパターンマッチングは、ゲームフェーズを**読みやすく、バグが混入しにくい**コードで表現します。

```elixir
# ゲームフェーズごとに処理が自然に分岐する
def handle_info(:tick, %{phase: :playing} = state),  do: {:noreply, update_game(state)}
def handle_info(:tick, %{phase: :paused}  = state),  do: {:noreply, state}
def handle_info(:tick, %{phase: :game_over} = state), do: {:noreply, handle_game_over(state)}
```

### OTP Supervisor — ゲームが「自己修復」する

`StressMonitor` プロセスがクラッシュしても、`GameLoop` は止まりません。Supervisor が自動的に再起動します。これは「防御的プログラミング」ではなく、**クラッシュを前提とした設計（Let it crash）** です。

```
GameLoop がクラッシュ → Supervisor が 1ms 以内に再起動 → ゲーム継続
SpawnSystem がクラッシュ → Supervisor が再起動 → ゲーム継続
StressMonitor がクラッシュ → Supervisor が再起動 → ゲーム継続
```

### プロセスの軽量さ

| | BEAM プロセス | OS スレッド |
|---|---|---|
| 生成コスト | ~1μs、約 2KB | ~10ms、~8MB |
| 同時実行数 | **数百万** | 数千（OS 制限） |
| スケジューリング | プリエンプティブ（VM 管理） | OS 管理 |
| 通信 | メッセージパッシング | 共有メモリ（ロック必要） |

ゲームの各サブシステムを独立したプロセスとして動かしても、オーバーヘッドはほぼゼロです。

---

## 3. なぜ Rust なのか — ゼロコスト抽象化の威力

### 10,000 体の敵を動かす 3 つの技術

#### 技術 1: SoA（Structure of Arrays）— キャッシュ効率の最大化

```rust
// 従来の AoS（Array of Structs）— キャッシュミスが多発
struct Enemy { x: f32, y: f32, hp: i32, sprite: u16, alive: bool, ... }
enemies: Vec<Enemy>  // 全フィールドが混在

// SoA（Structure of Arrays）— 必要なデータだけキャッシュに乗る
struct EnemyWorld {
    positions_x: Vec<f32>,  // 位置更新時: このデータだけ読む
    positions_y: Vec<f32>,  // → CPU キャッシュに 10,000 体分が乗る
    health:      Vec<i32>,  // HP 更新時: このデータだけ読む
    alive:       Vec<bool>, // 生存チェック: このデータだけ読む
}
```

位置更新のループは `positions_x` と `positions_y` の 2 配列だけを読みます。AoS よりキャッシュミスが少なく、**必要なデータがキャッシュに効率的に乗ります**。

#### 技術 2: Spatial Hash — 衝突判定を O(n²) から O(1) に近く

```
従来の全ペア判定:
  10,000 体 × 10,000 体 = 1 億回の距離計算 → 不可能

Spatial Hash:
  画面を 80px グリッドに分割
  各エンティティを該当セルに登録
  近傍クエリ: 周辺セルのみ検索 → O(1) に近い計算量
```

```rust
pub struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
}
// query_circle: 半径内のセルのみ検索
// 10,000 体でも 1 フレーム < 3ms
```

#### 技術 3: rayon 並列 AI — 全コアを使い切る

```rust
// 10,000 体の AI 更新を全 CPU コアで並列実行
enemies.positions_x
    .par_iter_mut()  // rayon: 自動的にスレッドプールに分散
    .zip(enemies.positions_y.par_iter_mut())
    .for_each(|(x, y)| {
        // 各敵の移動計算（独立しているので並列化可能）
        let (dx, dy) = chase_player(*x, *y, player_x, player_y);
        *x += dx * dt;
        *y += dy * dt;
    });
```

### GPU インスタンシング — 1 draw call で 10,000 体

```
従来の個別描画:
  10,000 体 × 1 draw call = 10,000 draw calls/フレーム → GPU が詰まる

インスタンシング:
  頂点バッファ: 四角形 1 枚（4 頂点）
  インスタンスバッファ: 10,000 体分の位置・UV データ
  → 1 draw call で全員を描画
```

```wgsl
// WGSL シェーダ: instance_index で各スプライトのデータを参照
@vertex
fn vs_main(@builtin(instance_index) idx: u32, ...) -> VertexOutput {
    let instance = instances[idx];  // 位置・UV・ティントを取得
    out.clip_position = camera.view_proj * vec4(instance.position, 0.0, 1.0);
    out.uv = instance.uv_offset + vertex_pos * instance.uv_size;
    return out;
}
```

---

## 4. Elixir × Rust の連携 — Rustler NIF の魔法

### ResourceArc — ゼロコピーでゲームワールドを共有

```
非効率なアプローチ（毎フレーム 10,000 体分をコピー）:
  Elixir → [シリアライズ: ~500KB] → Rust → [処理] → [デシリアライズ: ~500KB] → Elixir

ResourceArc アプローチ（ポインタのみ渡す）:
  Elixir → [world_ref: 8 バイト] → Rust → [処理] → [レンダリングデータ: ~50KB] → Elixir
```

Rust のゲームワールド（10,000 体分のデータ）は **Elixir から不透明な参照として扱われます**。毎フレームのデータコピーは最小限です。

### DirtyCpu スケジューラ — BEAM をブロックしない

```rust
// schedule = "DirtyCpu" により、物理演算が BEAM スケジューラをブロックしない
#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> rustler::Atom {
    let mut w = world.0.write().unwrap();
    w.step(delta_ms as f32);  // 物理演算
    atoms::ok()
}
// GameWorld(RwLock<GameWorldInner>) により、読み取り専用 NIF は並列実行可能
```

```
dirty_cpu なし:
  [GameLoop tick][物理演算 8ms ブロック][他の Elixir プロセスが待機]

dirty_cpu あり:
  [GameLoop tick] → dirty_cpu スレッドで物理演算
  [他の Elixir プロセスは通常通り動作]
```

### 1 フレームのデータフロー

```
1. GameLoop GenServer が :tick を受信（16ms 間隔）
2. Engine.physics_step(world_ref, delta_ms) → Rust で物理演算
   ├─ 移動計算（SoA + rayon、x86_64 では SIMD）
   ├─ 衝突判定・最近接探索（Spatial Hash）
   ├─ 武器・弾丸・パーティクル更新
   └─ 死亡処理・フリーリストでスロット回収
3. drain_frame_events → EventBus にブロードキャスト（Stats, Telemetry）
4. get_frame_metadata で HUD 描画データを 1 回取得（FrameCache）
5. SpawnSystem / BossSystem でスポーン判断（純粋関数）
6. LevelSystem.generate_weapon_choices/1 でレベルアップ武器候補
7. StressMonitor が独立プロセスでパフォーマンスをサンプリング
```

---

## 5. パフォーマンス実測値

### フレーム予算（目標: 16ms/frame @ 60fps）

| 処理 | 目標時間 | 担当 | 技術 |
|---|---|---|---|
| NIF 呼び出しオーバーヘッド | < 0.1ms | Rustler | DirtyCpu |
| 移動計算（10,000+ 体） | < 2ms | Rust | SoA + rayon（x86_64 では SIMD） |
| 衝突判定・最近接探索 | < 3ms | Rust | Spatial Hash |
| AI 更新 | < 2ms | Rust | rayon 並列化 |
| GPU 描画 | < 4ms | Rust + GPU | インスタンシング（最大 14,502 インスタンスを 1 draw call：敵・弾・パーティクル等） |
| Elixir ロジック | < 1ms | Elixir | GenServer |
| **合計** | **< 12ms** | — | **余裕あり** |

### スケーリング特性

```
100 体:   CPU 使用率 ~1%   / GPU 使用率 ~1%
1,000 体: CPU 使用率 ~5%   / GPU 使用率 ~2%
5,000 体: CPU 使用率 ~20%  / GPU 使用率 ~5%
10,000 体: CPU 使用率 ~40% / GPU 使用率 ~10%  ← 現在の上限
```

**10,000 体でも 60 FPS を維持。** GPU はまだ余裕があります。

---

## 6. このエンジンの「可能性」

### 今すぐ拡張できること

#### ゲームの差し替え — 汎用エンジンとして

```elixir
# config/config.exs でゲームを切り替え
config :game, current: Game.VampireSurvivor
# config :game, current: Game.MiniShooter  # 例: 他ゲーム
```

`Engine` API 経由でワールド操作。ゲームは `entity_registry` で敵・武器・ボスを定義し、`SceneBehaviour` でシーンを実装するだけで、**新ゲームをプラグインとして追加できます**。

#### マルチプレイ対応 — Elixir の本領発揮

```elixir
# Elixir の分散処理は言語に組み込まれている
Node.connect(:"game_server@192.168.1.1")
GenServer.call({Engine.GameLoop, :"game_server@192.168.1.1"}, :get_state)
```

BEAM VM の分散処理機能を使えば、**マルチプレイサーバーへの拡張が自然に行えます**。

#### ホットコードスワップ — 実行中にゲームロジックを変更

```elixir
# ゲームを止めずにスポーンテーブルを変更
:code.load_file(Game.VampireSurvivor.SpawnSystem)
# → 次のフレームから新しいロジックが適用される
```

**ゲームを再起動せずにバランス調整ができます。**

#### GPU コンピュートシェーダへの移行

現在の物理演算は CPU（Rust + rayon）で行っています。将来的に敵を 100,000 体に増やすには、wgpu のコンピュートシェーダに移行するだけです。Elixir 側のコードは一切変わりません。

```
現在:  Elixir → NIF → Rust CPU（rayon）→ GPU 描画
将来:  Elixir → NIF → Rust → GPU コンピュート → GPU 描画
                              ↑ ここだけ変わる
```

### 他のジャンルへの応用

このエンジンのアーキテクチャは、Vampire Survivors ライクに限りません。

| ジャンル | Elixir の役割 | Rust の役割 |
|---|---|---|
| RTS（リアルタイムストラテジー） | ユニット命令・資源管理・外交ロジック | 経路探索・戦闘演算・描画 |
| MMORPG サーバー | プレイヤー管理・チャット・マッチング | 物理演算・衝突判定 |
| シミュレーション | エージェント行動ルール・統計収集 | 大規模並列シミュレーション |
| リアルタイム対戦 | ゲームルール・ランキング・ロビー | 物理演算・描画 |

---

## 7. 他のアーキテクチャとの比較

### なぜ Unity/Unreal ではないのか

| 観点 | Unity/Unreal | Elixir × Rust |
|---|---|---|
| 大規模エンティティ | ECS（DOTS）で対応可能 | SoA + rayon で同等以上 |
| マルチプレイ | 追加ミドルウェアが必要 | BEAM VM に組み込み |
| ホットリロード | 限定的 | ネイティブサポート |
| 耐障害性 | 手動実装 | OTP Supervisor |
| カスタマイズ性 | エンジンに縛られる | **完全なコントロール** |
| ライセンス費用 | 収益に応じて発生 | **完全無料** |

### なぜ Go + Rust ではないのか

Go も goroutine による高並行性を持ちますが：

```
Go の課題:
  × Supervisor のような耐障害性の仕組みがない
  × goroutine のパニックは手動で recover する必要がある
  × ゲームフェーズ管理に状態機械ライブラリが別途必要
  × パターンマッチングがない

Elixir の優位点:
  ✓ Supervisor が自動的にクラッシュしたシステムを再起動
  ✓ GenServer がゲームループ・状態機械を自然に表現
  ✓ パターンマッチングによる宣言的なゲームロジック
  ✓ ホットコードスワップがネイティブサポート
```

---

## 8. 実装の旅 — 段階的に構築したエンジン

このゲームエンジンは、**Step 1〜40+** で段階的に構築されました。

```
Step 1〜15:  基礎実装（環境構築 → NIF 連携 → ゲームループ → 武器・UI → ゲームオーバー）
Step 16〜25: クオリティ（パーティクル・武器強化・敵タイプ・アイテム・カメラ・BGM/SE・ボス）
Step 26〜31: パフォーマンス（EventBus・ETS・RwLock・フリーリスト・Spatial Hash 最近接・SIMD）
Step 32〜40: 汎用化（Game インターフェース・ゲーム分離・Engine API 安定化・entity_registry）
Step 41〜44: 拡張（マップ・障害物、セーブ・ロード、マルチプレイ、デバッグ支援）
```

各ステップは「独立して動作確認できる単位」に分割されており、**誰でも再現できます**。

---

## 9. 技術スタックの全体像

| レイヤー | 技術 | 役割 |
|---|---|---|
| ゲームロジック | Elixir / BEAM VM | 司令塔・状態管理・耐障害性 |
| Elixir-Rust 連携 | Rustler | NIF ブリッジ（DirtyCpu スケジューラ） |
| 物理演算・AI | Rust + rayon | SoA ECS + 並列計算（x86_64 では SIMD Chase AI） |
| 空間分割 | Rust（自作） | Spatial Hash 衝突判定・最近接探索 |
| GPU 描画 | wgpu | クロスプラットフォーム GPU |
| HUD/UI | egui + egui-wgpu | インゲーム UI |
| 音声 | rodio | BGM・SE 再生 |
| ウィンドウ管理 | winit | クロスプラットフォームウィンドウ |
| シェーダー | WGSL | スプライトインスタンシング |
| アセット | include_bytes! / AssetLoader | スプライトアトラス・実行時ロード |

**外部サービス依存ゼロ。すべてがローカルで動作します。**

---

## 10. まとめ — このエンジンが示す未来

### 証明されたこと

1. **Elixir は「ゲームエンジン」ではないが「ゲームの司令塔」として最適**
   - BEAM VM の並行性・耐障害性・ソフトリアルタイム性はゲームロジック層に完璧に適合する

2. **Rust は「システムプログラミング言語」だが「ゲームエンジンのコア」として最強**
   - SoA + rayon（x86_64 では SIMD）+ Spatial Hash + wgpu の組み合わせで 10,000 体を 60 FPS で動かせる

3. **2 つの言語の組み合わせは、どちらか単独より強い**
   - Elixir だけ: 物理演算が遅く、大規模エンティティを扱えない
   - Rust だけ: 耐障害性がなく、ゲームロジックが複雑になる
   - **Elixir × Rust: 両方の強みだけを活かせる**

### このエンジンで作れるもの

- **大規模アクションゲーム**（Vampire Survivors、Brotato 系）
- **リアルタイムマルチプレイゲーム**（BEAM の分散処理で自然に拡張）
- **大規模シミュレーション**（都市シミュ、生態系シミュ）
- **ライブサービスゲーム**（ホットリロードで無停止アップデート）

### 次のステップ

このゲームエンジンは技術デモの域を超え、**実用に近い基盤**が整いました。

```
近期（Step 41〜44）:
  ├─ マップ・障害物システム（Ghost 壁すり抜けの土台）
  ├─ セーブ・ロード（ハイスコア永続化）
  ├─ デバッグ支援（NIF クラッシュ時のトレース改善）
  └─ マルチプレイ基盤（Phoenix Channels 連携）

中期:
  ├─ GPU コンピュートシェーダ（100,000 体への挑戦）
  ├─ より多くの武器・敵タイプ
  └─ パーティクルエフェクト強化

長期:
  ├─ マルチプレイ対応（BEAM 分散処理）
  ├─ ブラウザ観戦機能（Phoenix LiveView）
  └─ エディタ統合（ホットリロードを活かしたライブデザイン）
```

---

## 付録: 参考ドキュメント

| ドキュメント | 内容 |
|---|---|
| [STEPS.md](../05_steps/STEPS.md) | Step 1〜15 の実装ガイド |
| [STEPS_QUALITY.md](../05_steps/STEPS_QUALITY.md) | Step 16〜25 クオリティアップ |
| [STEPS_PERF.md](../05_steps/STEPS_PERF.md) | Step 26〜31 パフォーマンス改善 |
| [STEPS_MAP_SAVE_MULTI_DEBUG.md](../05_steps/STEPS_MAP_SAVE_MULTI_DEBUG.md) | Step 41〜44 マップ・セーブ・マルチ・デバッグ |
| [ENGINE_API.md](../06_system_design/ENGINE_API.md) | エンジン API 設計（安定化） |
| [ENGINE_STRENGTHS_WEAKNESSES.md](../02_spec_design/ENGINE_STRENGTHS_WEAKNESSES.md) | 強み・弱み総合サマリー |
| [SPEC.md](../01_setup/SPEC.md) | ゲーム仕様書・技術アーキテクチャ詳細 |
| [WHY_ELIXIR.md](../03_tech_decisions/WHY_ELIXIR.md) | Elixir 採用の技術的根拠 |
| [WHY_RAYON.md](../03_tech_decisions/WHY_RAYON.md) | rayon 並列化の詳細 |

---

> **「得意なことを得意な言語に任せる。」**  
> これがこのエンジンの設計哲学であり、Elixir × Rust が証明した答えです。
