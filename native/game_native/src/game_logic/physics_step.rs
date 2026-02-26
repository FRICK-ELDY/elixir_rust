//! Path: native/game_native/src/game_logic/physics_step.rs
//! Summary: 物理ステップ内部実装

use super::chase_ai::{find_nearest_enemy_spatial, find_nearest_enemy_spatial_excluding};
#[cfg(not(target_arch = "x86_64"))]
use super::chase_ai::update_chase_ai;
#[cfg(target_arch = "x86_64")]
use super::chase_ai::update_chase_ai_simd;
use super::systems::boss::update_boss;
use super::systems::collision::resolve_obstacles_enemy;
use super::systems::effects::{update_particles, update_score_popups};
use super::systems::items::update_items;
use super::systems::leveling::compute_weapon_choices;
use super::systems::projectiles::update_projectiles_and_enemy_hits;
use crate::world::{FrameEvent, GameWorldInner};
use crate::{BULLET_KIND_LIGHTNING, BULLET_KIND_WHIP};
use game_core::constants::{BULLET_LIFETIME, BULLET_SPEED, ENEMY_SEPARATION_FORCE, ENEMY_SEPARATION_RADIUS, FRAME_BUDGET_MS, INVINCIBLE_DURATION, MAP_HEIGHT, MAP_WIDTH, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED, WEAPON_SEARCH_RADIUS};
use game_core::entity_params::{garlic_radius, EnemyParams, WeaponParams, whip_range, lightning_chain_count, WEAPON_ID_AXE, WEAPON_ID_CROSS, WEAPON_ID_FIREBALL, WEAPON_ID_GARLIC, WEAPON_ID_LIGHTNING, WEAPON_ID_MAGIC_WAND, WEAPON_ID_WHIP};
use game_core::item::ItemKind;
use game_core::physics::obstacle_resolve;
use game_core::physics::separation::apply_separation;
use game_core::util::exp_required_for_next;

/// 1.5.1: 物理ステップの内部実装（NIF と Rust ゲームループスレッドの両方から呼ぶ）
pub(crate) fn physics_step_inner(w: &mut GameWorldInner, delta_ms: f64) {
    // trace にしておき、RUST_LOG=trace のときだけ毎フレーム出力（debug だと 60fps でコンソールが埋まる）
    log::trace!("physics_step: delta={}ms frame_id={}", delta_ms, w.frame_id);
    let t_start = std::time::Instant::now();

    w.frame_id += 1;

    let dt = delta_ms as f32 / 1000.0;

    // ── 1.7.5: スコアポップアップの lifetime を減衰 ──────────────
    update_score_popups(w, dt);

    // ── 1.1.13: 経過時間を更新 ──────────────────────────────────
    w.elapsed_seconds += dt;
    let dx = w.player.input_dx;
    let dy = w.player.input_dy;

    // 斜め移動を正規化して速度を一定に保つ
    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.001 {
        w.player.x += (dx / len) * PLAYER_SPEED * dt;
        w.player.y += (dy / len) * PLAYER_SPEED * dt;
    }

    // 1.5.2: プレイヤー vs 障害物（重なったら押し出し）
    obstacle_resolve::resolve_obstacles_player(
        &w.collision,
        &mut w.player.x,
        &mut w.player.y,
        &mut w.obstacle_query_buf,
    );

    w.player.x = w.player.x.clamp(0.0, MAP_WIDTH  - PLAYER_SIZE);
    w.player.y = w.player.y.clamp(0.0, MAP_HEIGHT - PLAYER_SIZE);

    // Chase AI（x86_64 では SIMD 版、それ以外は rayon 版）
    let px = w.player.x + PLAYER_RADIUS;
    let py = w.player.y + PLAYER_RADIUS;
    #[cfg(target_arch = "x86_64")]
    update_chase_ai_simd(&mut w.enemies, px, py, dt);
    #[cfg(not(target_arch = "x86_64"))]
    update_chase_ai(&mut w.enemies, px, py, dt);

    // 敵同士の重なりを解消する分離パス
    apply_separation(&mut w.enemies, ENEMY_SEPARATION_RADIUS, ENEMY_SEPARATION_FORCE, dt);

    // 1.5.2: 敵 vs 障害物（Ghost 以外は押し出し）
    resolve_obstacles_enemy(w);

    // ── 1.1.10: 衝突判定（Spatial Hash）────────────────────────
    // 1. 動的 Spatial Hash を再構築
    w.rebuild_collision();

    // 無敵タイマーを更新
    if w.player.invincible_timer > 0.0 {
        w.player.invincible_timer = (w.player.invincible_timer - dt).max(0.0);
    }

    // 2. プレイヤー周辺の敵を取得して円-円判定
    // 最大の敵半径（Golem: 32px）を考慮してクエリ半径を広げる
    let max_enemy_radius = 32.0_f32;
    let query_radius = PLAYER_RADIUS + max_enemy_radius;
    let candidates = w.collision.dynamic.query_nearby(px, py, query_radius);

    for idx in candidates {
        if !w.enemies.alive[idx] {
            continue;
        }
        let kind_id = w.enemies.kind_ids[idx];
        let params = EnemyParams::get(kind_id);
        let enemy_r = params.radius;
        let hit_radius = PLAYER_RADIUS + enemy_r;
        let ex = w.enemies.positions_x[idx] + enemy_r;
        let ey = w.enemies.positions_y[idx] + enemy_r;
        let ddx = px - ex;
        let ddy = py - ey;
        let dist_sq = ddx * ddx + ddy * ddy;

        if dist_sq < hit_radius * hit_radius {
            // 敵→プレイヤーへのダメージ（無敵時間中は無効）
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                let dmg = params.damage_per_sec * dt;
                w.player.hp = (w.player.hp - dmg).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
                w.frame_events.push(FrameEvent::PlayerDamaged { damage: dmg });
                // 赤いパーティクルをプレイヤー位置に発生
                let ppx = w.player.x + PLAYER_RADIUS;
                let ppy = w.player.y + PLAYER_RADIUS;
                w.particles.emit(ppx, ppy, 6, [1.0, 0.15, 0.15, 1.0]);
            }
        }
    }

    // ── 1.1.11/1.1.14/1.2.2/1.2.6: 武器スロット発射処理 ──────────────────
    // level_up_pending 中は発射を止めてゲームを一時停止する
    if !w.level_up_pending {
        // プレイヤーの移動方向（Whip の向き計算用）
        let facing_angle = {
            let fdx = w.player.input_dx;
            let fdy = w.player.input_dy;
            if fdx * fdx + fdy * fdy > 0.0001 {
                fdy.atan2(fdx)
            } else {
                // 停止中は右向きをデフォルトとする
                0.0_f32
            }
        };

        let slot_count = w.weapon_slots.len();
        for si in 0..slot_count {
            w.weapon_slots[si].cooldown_timer = (w.weapon_slots[si].cooldown_timer - dt).max(0.0);
            if w.weapon_slots[si].cooldown_timer > 0.0 {
                continue;
            }
            let kind_id = w.weapon_slots[si].kind_id;
            let wp = WeaponParams::get(kind_id);
            // 1.2.2: レベルに応じたクールダウン・ダメージ・弾数を使用
            let cd    = w.weapon_slots[si].effective_cooldown();
            let dmg   = w.weapon_slots[si].effective_damage();
            let level = w.weapon_slots[si].level;
            let bcount = w.weapon_slots[si].bullet_count();
            match kind_id {
                WEAPON_ID_MAGIC_WAND => {
                    if let Some(ti) = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS) {
                        let target_r = EnemyParams::get(w.enemies.kind_ids[ti]).radius;
                        let tx   = w.enemies.positions_x[ti] + target_r;
                        let ty   = w.enemies.positions_y[ti] + target_r;
                        let bdx  = tx - px;
                        let bdy  = ty - py;
                        // bcount 発同時発射（Lv3 で 2 発、Lv5 で 3 発）
                        // 複数発は少しずつ角度をずらして扇状に発射
                        let base_angle = bdy.atan2(bdx);
                        let spread = std::f32::consts::PI * 0.08; // 約 14 度の広がり
                        let half = (bcount as f32 - 1.0) / 2.0;
                        for bi in 0..bcount {
                            let angle = base_angle + (bi as f32 - half) * spread;
                            let vx = angle.cos() * BULLET_SPEED;
                            let vy = angle.sin() * BULLET_SPEED;
                            w.bullets.spawn(px, py, vx, vy, dmg, BULLET_LIFETIME, wp.as_u8);
                        }
                        w.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                WEAPON_ID_AXE => {
                    // 上方向に直進（簡易実装）
                    w.bullets.spawn(px, py, 0.0, -BULLET_SPEED, dmg, BULLET_LIFETIME, wp.as_u8);
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                WEAPON_ID_CROSS => {
                    // Lv1〜3: 上下左右 4 方向、Lv4 以上: 斜め 4 方向も追加
                    let dirs_4: [(f32, f32); 4] = [
                        (0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0),
                    ];
                    let diag = std::f32::consts::FRAC_1_SQRT_2;
                    let dirs_8: [(f32, f32); 8] = [
                        (0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0),
                        (diag, -diag), (-diag, -diag), (diag, diag), (-diag, diag),
                    ];
                    let dirs: &[(f32, f32)] = if bcount >= 8 { &dirs_8 } else { &dirs_4 };
                    for &(dx_dir, dy_dir) in dirs {
                        w.bullets.spawn(px, py, dx_dir * BULLET_SPEED, dy_dir * BULLET_SPEED, dmg, BULLET_LIFETIME, wp.as_u8);
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                // ── 1.2.6: Whip ──────────────────────────────────────────
                WEAPON_ID_WHIP => {
                    // プレイヤーの移動方向に扇状の判定を出す（弾丸を生成しない直接判定）
                    let whip_range = whip_range(kind_id, level);
                    let whip_half_angle = std::f32::consts::PI * 0.3; // 108度 / 2 = 54度
                    // facing_angle 方向の中間点にエフェクト弾を生成（kind=10: 黄緑の横長楕円）
                    let eff_x = px + facing_angle.cos() * whip_range * 0.5;
                    let eff_y = py + facing_angle.sin() * whip_range * 0.5;
                    w.bullets.spawn_effect(eff_x, eff_y, 0.12, BULLET_KIND_WHIP);
                    // 空間ハッシュで範囲内の候補のみ取得し、全敵ループを回避
                    let whip_range_sq = whip_range * whip_range;
                    let candidates = w.collision.dynamic.query_nearby(px, py, whip_range);
                    for ei in candidates {
                        if !w.enemies.alive[ei] { continue; }
                        let ex = w.enemies.positions_x[ei];
                        let ey = w.enemies.positions_y[ei];
                        let ddx = ex - px;
                        let ddy = ey - py;
                        // sqrt を避けて二乗比較で正確な円形クリップ
                        if ddx * ddx + ddy * ddy > whip_range_sq { continue; }
                        let angle = ddy.atan2(ddx);
                        // π/-π をまたぐ場合に正しく動作するよう -π〜π に正規化
                        let mut diff = angle - facing_angle;
                        if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                        if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                        if diff.abs() < whip_half_angle {
                            let enemy_r = EnemyParams::get(w.enemies.kind_ids[ei]).radius;
                            let hit_x = ex + enemy_r;
                            let hit_y = ey + enemy_r;
                            w.enemies.hp[ei] -= dmg as f32;
                            if w.enemies.hp[ei] <= 0.0 {
                                let kind_e = w.enemies.kind_ids[ei];
                                let ep_hit = EnemyParams::get(kind_e);
                                w.enemies.kill(ei);
                                w.kill_count += 1;
                                w.score_popups.push((hit_x, hit_y - 20.0, ep_hit.exp_reward * 2, 0.8));
                                w.frame_events.push(FrameEvent::EnemyKilled {
                                    enemy_kind:  kind_e,
                                    weapon_kind: wp.as_u8,
                                });
                                w.score += ep_hit.exp_reward * 2;
                                w.exp   += ep_hit.exp_reward;
                                if !w.level_up_pending {
                                    let required = exp_required_for_next(w.level);
                                    if w.exp >= required {
                                        let new_lv = w.level + 1;
                                        w.level_up_pending = true;
                                        w.weapon_choices = compute_weapon_choices(w);
                                        w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                                    }
                                }
                                w.particles.emit(hit_x, hit_y, 8, ep_hit.particle_color);
                                let roll = w.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (ItemKind::Potion, 20)
                                } else {
                                    (ItemKind::Gem, ep_hit.exp_reward)
                                };
                                w.items.spawn(hit_x, hit_y, item_kind, item_value);
                            } else {
                                w.particles.emit(hit_x, hit_y, 3, [1.0, 0.6, 0.1, 1.0]);
                            }
                        }
                    }
                    // 1.2.9: Whip vs ボス
                    {
                        let whip_range_sq = whip_range * whip_range;
                        let boss_hit_pos: Option<(f32, f32)> = if let Some(ref boss) = w.boss {
                            if !boss.invincible {
                                let ddx = boss.x - px;
                                let ddy = boss.y - py;
                                if ddx * ddx + ddy * ddy <= whip_range_sq {
                                    let angle = ddy.atan2(ddx);
                                    let mut diff = angle - facing_angle;
                                    if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                                    if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                                    if diff.abs() < whip_half_angle { Some((boss.x, boss.y)) } else { None }
                                } else { None }
                            } else { None }
                        } else { None };
                        if let Some((bx, by)) = boss_hit_pos {
                            if let Some(ref mut boss) = w.boss { boss.hp -= dmg as f32; }
                            w.particles.emit(bx, by, 4, [1.0, 0.8, 0.2, 1.0]);
                        }
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                WEAPON_ID_FIREBALL => {
                    // 最近接敵に向かって貫通弾を発射
                    if let Some(ti) = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS) {
                        let target_r = EnemyParams::get(w.enemies.kind_ids[ti]).radius;
                        let tx  = w.enemies.positions_x[ti] + target_r;
                        let ty  = w.enemies.positions_y[ti] + target_r;
                        let bdx = tx - px;
                        let bdy = ty - py;
                        let base_angle = bdy.atan2(bdx);
                        let vx = base_angle.cos() * BULLET_SPEED;
                        let vy = base_angle.sin() * BULLET_SPEED;
                        w.bullets.spawn_piercing(px, py, vx, vy, dmg, BULLET_LIFETIME, wp.as_u8);
                        w.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                // ── 1.2.6: Lightning ─────────────────────────────────────
                WEAPON_ID_LIGHTNING => {
                    // 最近接敵から始まり、最大 chain_count 体に連鎖
                    let chain_count = lightning_chain_count(kind_id, level);
                    // chain_count は最大 6 程度と小さいため Vec で十分（HashSet 不要）
                    let mut hit_vec: Vec<usize> = Vec::with_capacity(chain_count);
                    // 最初はプレイヤー位置から最近接敵を探す（空間ハッシュで候補を絞る）
                    let mut current = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS);
                    #[allow(unused_assignments)]
                    let mut next_search_x = px;
                    #[allow(unused_assignments)]
                    let mut next_search_y = py;
                    for _ in 0..chain_count {
                        if let Some(ei) = current {
                            let enemy_r = EnemyParams::get(w.enemies.kind_ids[ei]).radius;
                            let hit_x = w.enemies.positions_x[ei] + enemy_r;
                            let hit_y = w.enemies.positions_y[ei] + enemy_r;
                            w.enemies.hp[ei] -= dmg as f32;
                            // 電撃エフェクト弾（kind=9: 水色の電撃球）+ パーティクル
                            w.bullets.spawn_effect(hit_x, hit_y, 0.10, BULLET_KIND_LIGHTNING);
                            w.particles.emit(hit_x, hit_y, 5, [0.3, 0.8, 1.0, 1.0]);
                            if w.enemies.hp[ei] <= 0.0 {
                                let kind_e = w.enemies.kind_ids[ei];
                                let ep_chain = EnemyParams::get(kind_e);
                                w.enemies.kill(ei);
                                w.kill_count += 1;
                                w.score_popups.push((hit_x, hit_y - 20.0, ep_chain.exp_reward * 2, 0.8));
                                w.frame_events.push(FrameEvent::EnemyKilled {
                                    enemy_kind:  kind_e,
                                    weapon_kind: wp.as_u8,
                                });
                                w.score += ep_chain.exp_reward * 2;
                                w.exp   += ep_chain.exp_reward;
                                if !w.level_up_pending {
                                    let required = exp_required_for_next(w.level);
                                    if w.exp >= required {
                                        let new_lv = w.level + 1;
                                        w.level_up_pending = true;
                                        w.weapon_choices = compute_weapon_choices(w);
                                        w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                                    }
                                }
                                let roll = w.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (ItemKind::Potion, 20)
                                } else {
                                    (ItemKind::Gem, ep_chain.exp_reward)
                                };
                                w.items.spawn(hit_x, hit_y, item_kind, item_value);
                            }
                            hit_vec.push(ei);
                            next_search_x = hit_x;
                            next_search_y = hit_y;
                            current = find_nearest_enemy_spatial_excluding(
                                &w.collision, &w.enemies,
                                next_search_x, next_search_y,
                                WEAPON_SEARCH_RADIUS, &hit_vec,
                            );
                        } else {
                            break;
                        }
                    }
                    // 1.2.9: Lightning vs ボス（600px 以内なら連鎖先としてダメージ）
                    {
                        let boss_hit_pos: Option<(f32, f32)> = if let Some(ref boss) = w.boss {
                            if !boss.invincible {
                                let ddx = boss.x - px;
                                let ddy = boss.y - py;
                                if ddx * ddx + ddy * ddy < 600.0 * 600.0 {
                                    Some((boss.x, boss.y))
                                } else { None }
                            } else { None }
                        } else { None };
                        if let Some((bx, by)) = boss_hit_pos {
                            if let Some(ref mut boss) = w.boss { boss.hp -= dmg as f32; }
                            w.bullets.spawn_effect(bx, by, 0.10, BULLET_KIND_LIGHTNING);
                            w.particles.emit(bx, by, 5, [0.3, 0.8, 1.0, 1.0]);
                        }
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                WEAPON_ID_GARLIC => {
                    // プレイヤー周囲オーラで一定間隔ダメージ（5 dmg/sec 想定: 0.2s 毎に 1）
                    let radius = garlic_radius(kind_id, level);
                    let radius_sq = radius * radius;
                    let candidates = w.collision.dynamic.query_nearby(px, py, radius);
                    for ei in candidates {
                        if !w.enemies.alive[ei] { continue; }
                        let ex = w.enemies.positions_x[ei];
                        let ey = w.enemies.positions_y[ei];
                        let ddx = ex - px;
                        let ddy = ey - py;
                        if ddx * ddx + ddy * ddy > radius_sq { continue; }
                        w.enemies.hp[ei] -= dmg as f32;
                        let kind_e = w.enemies.kind_ids[ei];
                        let ep = EnemyParams::get(kind_e);
                        let hit_x = ex + ep.radius;
                        let hit_y = ey + ep.radius;
                        if w.enemies.hp[ei] <= 0.0 {
                            w.enemies.kill(ei);
                            w.kill_count += 1;
                            w.score_popups.push((hit_x, hit_y - 20.0, ep.exp_reward * 2, 0.8));
                            w.frame_events.push(FrameEvent::EnemyKilled {
                                enemy_kind: kind_e,
                                weapon_kind: wp.as_u8,
                            });
                            w.score += ep.exp_reward * 2;
                            w.exp += ep.exp_reward;
                            if !w.level_up_pending {
                                let required = exp_required_for_next(w.level);
                                if w.exp >= required {
                                    w.level_up_pending = true;
                                    w.weapon_choices = compute_weapon_choices(w);
                                    w.frame_events.push(FrameEvent::LevelUp { new_level: w.level + 1 });
                                }
                            }
                            w.particles.emit(hit_x, hit_y, 8, ep.particle_color);
                            let roll = w.rng.next_u32() % 100;
                            let (item_kind, item_value) = if roll < 2 {
                                (ItemKind::Magnet, 0)
                            } else if roll < 7 {
                                (ItemKind::Potion, 20)
                            } else {
                                (ItemKind::Gem, ep.exp_reward)
                            };
                            w.items.spawn(hit_x, hit_y, item_kind, item_value);
                        } else {
                            w.particles.emit(hit_x, hit_y, 2, [0.9, 0.9, 0.3, 0.6]);
                        }
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                _ => {} // 未知の武器 ID（7 以上）は何もしない
            }
        }
    }

    // ── パーティクル更新: 移動 + 重力 + フェードアウト ───────────
    update_particles(w, dt);

    // ── 1.2.4: アイテム更新（磁石エフェクト + 自動収集） ─────
    update_items(w, dt, px, py);

    // ── 弾丸移動 + 弾丸 vs 敵衝突判定 ───────────────────────────
    update_projectiles_and_enemy_hits(w, dt);

    // ── 1.2.9: ボス更新 ─────────────────────────────────────────
    update_boss(w, dt);

    // ── 1.1.12: フレーム時間計測 ────────────────────────────────
    let elapsed_ms = t_start.elapsed().as_secs_f64() * 1000.0;
    w.last_frame_time_ms = elapsed_ms;
    if elapsed_ms > FRAME_BUDGET_MS {
        eprintln!(
            "[PERF] Frame budget exceeded: {:.2}ms (enemies: {})",
            elapsed_ms,
            w.enemies.count
        );
    }
}
