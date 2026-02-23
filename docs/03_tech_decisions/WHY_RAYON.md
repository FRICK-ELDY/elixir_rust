# なぜ rayon を並列化ライブラリに選ぶのか

このドキュメントでは、Chase AI（敵の追跡移動計算）の並列化に rayon を採用した技術的根拠を説明します。

---

## 目次

1. [rayon とは](#1-rayon-とは)
2. [採用の背景：5000体問題](#2-採用の背景5000体問題)
3. [内部の仕組み：Work-Stealing スレッドプール](#3-内部の仕組みwork-stealing-スレッドプール)
4. [SoA レイアウトとの相乗効果](#4-soa-レイアウトとの相乗効果)
5. [実装：最小限の変更で並列化](#5-実装最小限の変更で並列化)
6. [実測結果](#6-実測結果)
7. [他の並列化手段との比較](#7-他の並列化手段との比較)
8. [rayon が苦手なケースと注意点](#8-rayon-が苦手なケースと注意点)

---

## 1. rayon とは

rayon は Rust のデータ並列処理ライブラリです。通常のイテレータを**並列イテレータ**に変換するだけで、CPU の全コアを自動的に活用できます。

```rust
// 変更前（シングルスレッド）
vec.iter_mut().for_each(|x| { /* 処理 */ });

// 変更後（全コア並列）
vec.par_iter_mut().for_each(|x| { /* 処理 */ });
```

`par_iter()` に変えるだけという**最小限の変更**でマルチコア並列化が実現できる点が最大の特徴です。

---

## 2. 採用の背景：5000体問題

Step 12 の目標は「5000体の敵を 60fps（16ms 以内）で動かす」ことです。

### シングルスレッドでのボトルネック分析

```
1フレーム（16ms）の内訳:
  Chase AI 移動計算   : O(N) ← 敵数に比例して増加
  Spatial Hash 再構築 : O(N)
  衝突判定           : O(N × 近傍数)
  弾丸処理           : O(B)（弾丸数）
```

敵数が増えるほど Chase AI の計算コストが支配的になります。しかし Chase AI の各敵の計算は**完全に独立**しており、他の敵の状態を参照しません。これはデータ並列化の理想的な条件です。

---

## 3. 内部の仕組み：Work-Stealing スレッドプール

rayon は内部に**ワークスティーリング方式**のスレッドプールを持ちます。

```
CPU コア数 = 8 の場合:

  rayon スレッドプール（起動時に 1 度だけ生成）
  ┌─────────────────────────────────────────┐
  │ Thread 1: [敵 0   〜 1249]  ← 実行中   │
  │ Thread 2: [敵 1250〜 2499]  ← 実行中   │
  │ Thread 3: [敵 2500〜 3749]  ← 実行中   │
  │ Thread 4: [敵 3750〜 4999]  ← 実行中   │
  │ Thread 5: [敵 5000〜 6249]  ← 実行中   │
  │ Thread 6: [敵 6250〜 7499]  ← 実行中   │
  │ Thread 7: [敵 7500〜 8749]  ← 実行中   │
  │ Thread 8: [敵 8750〜 9999]  ← 実行中   │
  └─────────────────────────────────────────┘
```

### ワークスティーリングの利点

各スレッドは自分のキューが空になると、**他のスレッドのキューからタスクを「盗む」**ことで自動的に負荷分散します。敵の計算量にばらつきがあっても（alive チェックで早期 return するケースなど）、CPU コアが遊ぶことなく効率よく処理されます。

### スレッドプールの再利用

rayon のスレッドプールはプログラム起動時に**1 度だけ生成**されます。毎フレーム `par_iter()` を呼んでもスレッド生成コストは発生しません。

---

## 4. SoA レイアウトとの相乗効果

本プロジェクトの敵データは **SoA（Structure of Arrays）** レイアウトを採用しています。これが rayon の並列化と組み合わさることで、キャッシュ効率と並列効率の両方を最大化します。

### AoS vs SoA

```
AoS（Array of Structs）← 採用しなかった場合
メモリ配置:
  [x,y,vx,vy,spd,hp,alive] [x,y,vx,vy,spd,hp,alive] ...
  ↑ 移動計算で x だけ読みたいのに、不要なフィールドもキャッシュラインに載る

SoA（Struct of Arrays）← 本プロジェクトの実装
メモリ配置:
  positions_x:  [x0, x1, x2, x3, ...]  ← 連続
  positions_y:  [y0, y1, y2, y3, ...]  ← 連続
  velocities_x: [vx0, vx1, vx2, ...]   ← 連続
  ...
  ↑ 移動計算に必要なデータだけが連続して並ぶ → キャッシュヒット率が最大
```

### 並列化との組み合わせ

SoA では各配列を独立したスライスとして取り出せるため、rayon の `into_par_iter()` で複数スライスを `zip` して並列処理できます。

```rust
(positions_x, positions_y, velocities_x, velocities_y, speeds, alive)
    .into_par_iter()
    .for_each(|(px, py, vx, vy, speed, is_alive)| {
        // 各スレッドが独立したキャッシュラインを操作
        // → False Sharing が発生しにくい
    });
```

---

## 5. 実装：最小限の変更で並列化

### 変更前（シングルスレッド）

```rust
pub fn update_chase_ai(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    for i in 0..enemies.len() {
        if !enemies.alive[i] { continue; }
        let dx   = player_x - enemies.positions_x[i];
        let dy   = player_y - enemies.positions_y[i];
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        enemies.velocities_x[i] = (dx / dist) * enemies.speeds[i];
        enemies.velocities_y[i] = (dy / dist) * enemies.speeds[i];
        enemies.positions_x[i] += enemies.velocities_x[i] * dt;
        enemies.positions_y[i] += enemies.velocities_y[i] * dt;
    }
}
```

### 変更後（rayon 並列化）

```rust
use rayon::prelude::*;

pub fn update_chase_ai(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    let len = enemies.len();
    let positions_x  = &mut enemies.positions_x[..len];
    let positions_y  = &mut enemies.positions_y[..len];
    let velocities_x = &mut enemies.velocities_x[..len];
    let velocities_y = &mut enemies.velocities_y[..len];
    let speeds       = &enemies.speeds[..len];
    let alive        = &enemies.alive[..len];

    (positions_x, positions_y, velocities_x, velocities_y, speeds, alive)
        .into_par_iter()
        .for_each(|(px, py, vx, vy, speed, is_alive)| {
            if !*is_alive { return; }
            let dx   = player_x - *px;
            let dy   = player_y - *py;
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            *vx  = (dx / dist) * speed;
            *vy  = (dy / dist) * speed;
            *px += *vx * dt;
            *py += *vy * dt;
        });
}
```

### コンパイル時の安全性保証

rayon は `Send + Sync` トレイト境界によって、**データ競合が発生しないことをコンパイル時に保証**します。各要素への可変参照が重複しないことを Rust の借用チェッカーが静的に検証するため、実行時のロックが不要です。

---

## 6. 実測結果

| 敵の数 | PhysicsTime（実測） | 16ms 予算に対する割合 |
|---|---|---|
| 5,000 体 | 0.34ms | 約 2% |
| 10,000 体 | 0.41〜0.72ms | 約 3〜4% |

10,000 体でも 16ms フレーム予算の **4% 以下**に収まり、残り 96% のフレーム予算を衝突判定・弾丸処理・描画に使えます。

### なぜこれほど速いのか

| 要因 | 効果 |
|---|---|
| **rayon 並列化** | CPU コア数に比例してスケール |
| **SoA キャッシュ効率** | L1/L2 キャッシュのヒット率が最大化 |
| **計算の単純さ** | 四則演算 + sqrt のみ → CPU スループットが高い |
| **`DirtyCpu` NIF** | Erlang VM スケジューラを妨げない専用スレッドで実行 |
| **スレッドプール再利用** | 毎フレームのスレッド生成コストがゼロ |

---

## 7. 他の並列化手段との比較

| 手段 | 実装コスト | 安全性 | 適用場面 |
|---|---|---|---|
| **rayon** | 低（`par_iter` に変えるだけ） | コンパイル時保証 | データ並列（今回の用途） |
| `std::thread` | 高（手動でスレッド管理） | 実行時チェック | タスク並列 |
| `tokio` | 中（async/await） | コンパイル時保証 | I/O バウンド処理 |
| SIMD（`std::simd`） | 非常に高（手動ベクトル化） | unsafe 必要 | 4〜8倍の高速化が必要な場合 |
| シングルスレッド最適化 | 中（アルゴリズム改善） | 高 | データ数が少ない場合 |

rayon は「実装コストが最も低く、安全性が最も高い」という点で、今回のユースケースに最適な選択です。

---

## 8. rayon が苦手なケースと注意点

rayon が常に速いわけではありません。

### 苦手なケース

| ケース | 理由 |
|---|---|
| **データ数が少ない**（〜数百体） | スレッド間の同期コストが計算コストを上回る |
| **データ間に依存関係がある** | 並列化できない（コンパイルエラーになる） |
| **I/O バウンド処理** | CPU を使わないので並列化の恩恵がない |
| **頻繁なメモリ確保** | アロケータがボトルネックになる |

### Spatial Hash・衝突判定に rayon を使わない理由

衝突判定フェーズでは、複数の敵が同じプレイヤーの HP を同時に書き換える可能性があります。これはデータ競合が発生するため、rayon での並列化には追加の同期機構（`Mutex` や `AtomicF32` など）が必要になり、かえってオーバーヘッドが増えます。

```
Chase AI:    各敵は独立 → rayon で並列化 ✓
衝突判定:   複数敵が同じプレイヤー HP を書き換える可能性 → 直列処理 ✓
```

### Erlang dirty_cpu スケジューラとの関係

rayon のスレッドプールは Erlang VM の `dirty_cpu` スケジューラとは独立して動作します。`physics_step` NIF は `DirtyCpu` スケジューラで実行されますが、その内部で rayon が追加のスレッドを使うため、実際には **dirty_cpu スレッド数 + rayon スレッド数** 分の CPU コアを消費します。CPU コア数が少ない環境では、`RAYON_NUM_THREADS` 環境変数でスレッド数を制限することを検討してください。

---

## まとめ

rayon を採用した理由は以下の 3 点に集約されます。

1. **実装コストが最小** — `par_iter()` への変更だけで並列化が完了する
2. **安全性が最高** — Rust のコンパイル時チェックにより、データ競合が原理的に発生しない
3. **SoA との相乗効果** — キャッシュ効率と並列効率が同時に最大化される

結果として、10,000 体の敵を **0.72ms 以下**（16ms フレーム予算の 4%）で処理するパフォーマンスを達成しました。
