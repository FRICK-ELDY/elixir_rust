//! Path: native/game_native/src/game_logic/physics_step.rs
//! Summary: 物理ステップ内部実装

use super::chase_ai::{find_nearest_enemy_spatial, find_nearest_enemy_spatial_excluding, update_chase_ai, update_chase_ai_simd};
use crate::world::{FrameEvent, GameWorldInner};
use crate::{BULLET_KIND_LIGHTNING, BULLET_KIND_ROCK, BULLET_KIND_WHIP};
use game_core::constants::{BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED, ENEMY_SEPARATION_FORCE, ENEMY_SEPARATION_RADIUS, FRAME_BUDGET_MS, INVINCIBLE_DURATION, MAP_HEIGHT, MAP_WIDTH, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED, SCREEN_HEIGHT, SCREEN_WIDTH, WEAPON_SEARCH_RADIUS};
use game_core::entity_params::{garlic_radius, BossParams, EnemyParams, WeaponParams, whip_range, lightning_chain_count, BOSS_ID_BAT_LORD, BOSS_ID_SLIME_KING, BOSS_ID_STONE_GOLEM, WEAPON_ID_AXE, WEAPON_ID_CROSS, WEAPON_ID_FIREBALL, WEAPON_ID_GARLIC, WEAPON_ID_LIGHTNING, WEAPON_ID_MAGIC_WAND, WEAPON_ID_WHIP};
use game_core::item::ItemKind;
use game_core::physics::obstacle_resolve;
use game_core::physics::separation::apply_separation;
use game_core::util::{exp_required_for_next, spawn_position_around_player};

/// 1.7.5: レベルアップ時の武器選択肢を計算（未所持優先 → 低レベル順、Lv8 除外）
fn compute_weapon_choices(w: &GameWorldInner) -> Vec<String> {
    const ALL: &[(&str, u8)] = &[
        ("magic_wand", 0), ("axe", 1), ("cross", 2),
        ("whip", 3), ("fireball", 4), ("lightning", 5),
    ];
    let mut choices: Vec<(i32, String)> = ALL.iter()
        .filter_map(|(name, wid)| {
            let lv = w.weapon_slots.iter()
                .find(|s| s.kind_id == *wid)
                .map(|s| s.level)
                .unwrap_or(0);
            if lv >= 8 { return None; }
            let sort_key = if lv == 0 { -1i32 } else { lv as i32 };
            Some((sort_key, (*name).to_string()))
        })
        .collect();
    choices.sort_by_key(|(k, _)| *k);
    choices.into_iter().take(3).map(|(_, n)| n).collect()
}

/// プレイヤー周囲 800〜1200px の円周上にスポーン位置を生成（spawn_enemies / spawn_elite_enemy 共通）
pub(crate) fn get_spawn_positions_around_player(w: &mut GameWorldInner, count: usize) -> Vec<(f32, f32)> {
    let px = w.player.x + PLAYER_RADIUS;
    let py = w.player.y + PLAYER_RADIUS;
    (0..count)
        .map(|_| spawn_position_around_player(&mut w.rng, px, py, 800.0, 1200.0))
        .collect()
}

/// 1.5.2: 敵が障害物と重なっている場合に押し出す（Ghost はスキップ）
fn resolve_obstacles_enemy(w: &mut GameWorldInner) {
    let collision = &w.collision;
    let buf = &mut w.obstacle_query_buf;
    for i in 0..w.enemies.len() {
        if !w.enemies.alive[i] || EnemyParams::passes_through_obstacles(w.enemies.kind_ids[i]) {
            continue;
        }
        let r = EnemyParams::get(w.enemies.kind_ids[i]).radius;
        let cx = w.enemies.positions_x[i] + r;
        let cy = w.enemies.positions_y[i] + r;
        collision.query_static_nearby_into(cx, cy, r, buf);
        for &idx in buf.iter() {
            if let Some(o) = collision.obstacles.get(idx) {
                let dx = cx - o.x;
                let dy = cy - o.y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                let overlap = (r + o.radius) - dist;
                if overlap > 0.0 {
                    w.enemies.positions_x[i] += (dx / dist) * overlap;
                    w.enemies.positions_y[i] += (dy / dist) * overlap;
                }
            }
        }
    }
}

/// 1.5.1: 物理ステップの内部実装（NIF と Rust ゲームループスレッドの両方から呼ぶ）
pub(crate) fn physics_step_inner(w: &mut GameWorldInner, delta_ms: f64) {
    // trace にしておき、RUST_LOG=trace のときだけ毎フレーム出力（debug だと 60fps でコンソールが埋まる）
    log::trace!("physics_step: delta={}ms frame_id={}", delta_ms, w.frame_id);
    let t_start = std::time::Instant::now();

    w.frame_id += 1;

    let dt = delta_ms as f32 / 1000.0;

    // ── 1.7.5: スコアポップアップの lifetime を減衰 ──────────────
    for (_, _, _, lt) in w.score_popups.iter_mut() {
        *lt -= dt;
    }
    w.score_popups.retain(|(_, _, _, lt)| *lt > 0.0);

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
    {
        let plen = w.particles.len();
        for i in 0..plen {
            if !w.particles.alive[i] { continue; }
            w.particles.positions_x[i] += w.particles.velocities_x[i] * dt;
            w.particles.positions_y[i] += w.particles.velocities_y[i] * dt;
            w.particles.velocities_y[i] += 200.0 * dt;
            w.particles.lifetime[i] -= dt;
            if w.particles.lifetime[i] <= 0.0 {
                w.particles.kill(i);
            }
        }
    }

    // ── 1.2.4: アイテム更新（磁石エフェクト + 自動収集） ─────
    {
        // 磁石タイマー更新
        if w.magnet_timer > 0.0 {
            w.magnet_timer = (w.magnet_timer - dt).max(0.0);
        }

        // 磁石エフェクト: アクティブ中は宝石がプレイヤーに向かって飛んでくる
        if w.magnet_timer > 0.0 {
            let item_len = w.items.len();
            for i in 0..item_len {
                if !w.items.alive[i] { continue; }
                if w.items.kinds[i] != ItemKind::Gem { continue; }
                let dx = px - w.items.positions_x[i];
                let dy = py - w.items.positions_y[i];
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                w.items.positions_x[i] += (dx / dist) * 300.0 * dt;
                w.items.positions_y[i] += (dy / dist) * 300.0 * dt;
            }
        }

        // 自動収集判定（通常: 60px、磁石中: 全画面）
        let collect_r = if w.magnet_timer > 0.0 { 9999.0_f32 } else { 60.0_f32 };
        let collect_r_sq = collect_r * collect_r;
        let item_len = w.items.len();
        for i in 0..item_len {
            if !w.items.alive[i] { continue; }
            let dx = px - w.items.positions_x[i];
            let dy = py - w.items.positions_y[i];
            if dx * dx + dy * dy <= collect_r_sq {
                let item_k = w.items.kinds[i];
                match item_k {
                    ItemKind::Gem => {
                        // EXP は既に撃破時に加算済みのため、ここでは収集のみ
                    }
                    ItemKind::Potion => {
                        // HP 回復（最大 HP を超えない）
                        w.player.hp = (w.player.hp + w.items.value[i] as f32)
                            .min(w.player_max_hp);
                        // 回復パーティクル（緑）
                        w.particles.emit(px, py, 6, [0.2, 1.0, 0.4, 1.0]);
                    }
                    ItemKind::Magnet => {
                        // 磁石エフェクトを 10 秒間有効化
                        w.magnet_timer = 10.0;
                        // 磁石パーティクル（黄）
                        w.particles.emit(px, py, 8, [1.0, 0.9, 0.2, 1.0]);
                    }
                }
                w.frame_events.push(FrameEvent::ItemPickup { item_kind: item_k as u8 });
                w.items.kill(i);
            }
        }
    }

    // 2. 弾丸を移動・寿命更新
    let bullet_len = w.bullets.len();
    for i in 0..bullet_len {
        if !w.bullets.alive[i] {
            continue;
        }
        w.bullets.positions_x[i] += w.bullets.velocities_x[i] * dt;
        w.bullets.positions_y[i] += w.bullets.velocities_y[i] * dt;
        w.bullets.lifetime[i]    -= dt;
        if w.bullets.lifetime[i] <= 0.0 {
            w.bullets.kill(i);
            continue;
        }
        // 1.5.2: 障害物に当たったら弾を消す
        let bx = w.bullets.positions_x[i];
        let by = w.bullets.positions_y[i];
        w.collision.query_static_nearby_into(bx, by, BULLET_RADIUS, &mut w.obstacle_query_buf);
        if !w.obstacle_query_buf.is_empty() {
            w.bullets.kill(i);
            continue;
        }
        // 画面外に出た弾丸も消す
        if bx < -100.0 || bx > MAP_WIDTH + 100.0 || by < -100.0 || by > MAP_HEIGHT + 100.0 {
            w.bullets.kill(i);
        }
    }

    // 3. 弾丸 vs 敵 衝突判定
    // 最大の敵半径（Golem: 32px）を考慮してクエリ半径を広げる
    let bullet_query_r = BULLET_RADIUS + 32.0_f32;
    for bi in 0..bullet_len {
        if !w.bullets.alive[bi] {
            continue;
        }
        let dmg = w.bullets.damage[bi];
        // ダメージ 0 はエフェクト専用弾（Whip / Lightning）— 衝突判定をスキップ
        if dmg == 0 {
            continue;
        }
        let bx       = w.bullets.positions_x[bi];
        let by       = w.bullets.positions_y[bi];
        let piercing = w.bullets.piercing[bi];

        let nearby = w.collision.dynamic.query_nearby(bx, by, bullet_query_r);
        for ei in nearby {
            if !w.enemies.alive[ei] {
                continue;
            }
            let kind_id = w.enemies.kind_ids[ei];
            let ep = EnemyParams::get(kind_id);
            let enemy_r = ep.radius;
            let hit_r   = BULLET_RADIUS + enemy_r;
            let ex  = w.enemies.positions_x[ei] + enemy_r;
            let ey  = w.enemies.positions_y[ei] + enemy_r;
            let ddx = bx - ex;
            let ddy = by - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                w.enemies.hp[ei] -= dmg as f32;
                if w.enemies.hp[ei] <= 0.0 {
                    let weapon_k = w.bullets.weapon_kind[bi];
                    w.enemies.kill(ei);
                    w.kill_count += 1;
                    w.score_popups.push((ex, ey - 20.0, ep.exp_reward * 2, 0.8));
                    w.frame_events.push(FrameEvent::EnemyKilled {
                        enemy_kind:  kind_id,
                        weapon_kind: weapon_k,
                    });
                    // ── 1.1.13: 敵撃破でスコア加算 ──────────────
                    // 1.2.3: 敵タイプに応じたスコア（経験値 × 2）
                    w.score += ep.exp_reward * 2;
                    // ── 1.1.14/1.2.3: 経験値加算（タイプ別）────────
                    w.exp += ep.exp_reward;
                    if !w.level_up_pending {
                        let required = exp_required_for_next(w.level);
                        if w.exp >= required {
                            let new_lv = w.level + 1;
                            w.level_up_pending = true;
                            w.weapon_choices = compute_weapon_choices(w);
                            w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                        }
                    }
                    // ── 1.2.1/1.2.3: 敵タイプ別パーティクル ────────
                    w.particles.emit(ex, ey, 8, ep.particle_color);
                    // ── 1.2.4: アイテムドロップ（1体につき最大1種類）──
                    // 0〜1%: 磁石、2〜6%: 回復ポーション、7〜100%: 経験値宝石
                    let roll = w.rng.next_u32() % 100;
                    let (item_kind, item_value) = if roll < 2 {
                        (ItemKind::Magnet, 0)
                    } else if roll < 7 {
                        (ItemKind::Potion, 20)
                    } else {
                        (ItemKind::Gem, ep.exp_reward)
                    };
                    w.items.spawn(ex, ey, item_kind, item_value);
                } else {
                    // ── 1.2.1: ヒット時黄色パーティクル ─────────
                    // ── 1.2.6: Fireball は炎色パーティクル ──────
                    let hit_color = if piercing {
                        [1.0, 0.4, 0.0, 1.0]  // 炎（橙赤）
                    } else {
                        [1.0, 0.9, 0.3, 1.0]  // 通常（黄）
                    };
                    w.particles.emit(ex, ey, 3, hit_color);
                }
                // 貫通弾は消えない、通常弾は消す
                if !piercing {
                    w.bullets.kill(bi);
                    break;
                }
            }
        }
    }

    // ── 1.2.9: ボス更新（Elixir が spawn_boss で生成したボスを毎フレーム動かす）
    {
        // 借用競合を避けるため、副作用データを先に収集する
        struct BossEffect {
            spawn_slimes:    bool,
            spawn_rocks:     bool,
            bat_dash:        bool,
            special_x:       f32,
            special_y:       f32,
            hurt_player:     bool,
            hurt_x:          f32,
            hurt_y:          f32,
            boss_damage:     f32,
            bullet_hits:     Vec<(usize, f32, bool)>,  // (bullet_idx, dmg, kill_bullet)
            boss_x:          f32,
            boss_y:          f32,
            boss_invincible: bool,
            boss_r:          f32,
            boss_exp_reward: u32,
            boss_hp_ref:     f32,
            boss_killed:     bool,
            exp_reward:      u32,
            kill_x:          f32,
            kill_y:          f32,
        }
        let mut eff = BossEffect {
            spawn_slimes: false, spawn_rocks: false, bat_dash: false,
            special_x: 0.0, special_y: 0.0,
            hurt_player: false, hurt_x: 0.0, hurt_y: 0.0,
            boss_damage: 0.0,
            bullet_hits: Vec::new(),
            boss_x: 0.0, boss_y: 0.0,
            boss_invincible: false, boss_r: 0.0, boss_exp_reward: 0, boss_hp_ref: 0.0,
            boss_killed: false, exp_reward: 0, kill_x: 0.0, kill_y: 0.0,
        };

        // ── フェーズ1: boss の移動・タイマー更新（boss のみを借用）──
        if w.boss.is_some() {
            // プレイヤー座標をコピーして boss 借用前に取得
            let px = w.player.x + PLAYER_RADIUS;
            let py = w.player.y + PLAYER_RADIUS;

            let boss = w.boss.as_mut().unwrap();

            // 無敵タイマー
            if boss.invincible_timer > 0.0 {
                boss.invincible_timer = (boss.invincible_timer - dt).max(0.0);
                if boss.invincible_timer <= 0.0 { boss.invincible = false; }
            }

            // 移動 AI
            let bp = BossParams::get(boss.kind_id);
            match boss.kind_id {
                BOSS_ID_SLIME_KING | BOSS_ID_STONE_GOLEM => {
                    let ddx = px - boss.x;
                    let ddy = py - boss.y;
                    let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                    let spd = bp.speed;
                    boss.x += (ddx / dist) * spd * dt;
                    boss.y += (ddy / dist) * spd * dt;
                }
                BOSS_ID_BAT_LORD => {
                    if boss.is_dashing {
                        boss.x += boss.dash_vx * dt;
                        boss.y += boss.dash_vy * dt;
                        boss.dash_timer -= dt;
                        if boss.dash_timer <= 0.0 {
                            boss.is_dashing = false;
                            boss.invincible = false;
                            boss.invincible_timer = 0.0;
                        }
                    } else {
                        let ddx = px - boss.x;
                        let ddy = py - boss.y;
                        let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                        boss.x += (ddx / dist) * bp.speed * dt;
                        boss.y += (ddy / dist) * bp.speed * dt;
                    }
                }
                _ => {}
            }
            boss.x = boss.x.clamp(bp.radius, SCREEN_WIDTH  - bp.radius);
            boss.y = boss.y.clamp(bp.radius, SCREEN_HEIGHT - bp.radius);

            // 特殊行動タイマー
            boss.phase_timer -= dt;
            if boss.phase_timer <= 0.0 {
                boss.phase_timer = bp.special_interval;
                match boss.kind_id {
                    BOSS_ID_SLIME_KING => {
                        eff.spawn_slimes = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                    BOSS_ID_BAT_LORD => {
                        let ddx = px - boss.x;
                        let ddy = py - boss.y;
                        let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                        boss.dash_vx = (ddx / dist) * 500.0;
                        boss.dash_vy = (ddy / dist) * 500.0;
                        boss.is_dashing = true;
                        boss.dash_timer = 0.6;
                        boss.invincible = true;
                        boss.invincible_timer = 0.6;
                        eff.bat_dash = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                    BOSS_ID_STONE_GOLEM => {
                        eff.spawn_rocks = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                    _ => {}
                }
            }

            // ボス vs プレイヤー接触ダメージ: フラグだけ立てる
            let boss_r = bp.radius;
            let hit_r  = PLAYER_RADIUS + boss_r;
            let ddx = px - boss.x;
            let ddy = py - boss.y;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                eff.hurt_player = true;
                eff.hurt_x = px;
                eff.hurt_y = py;
                eff.boss_damage = bp.damage_per_sec;
            }

            // 弾丸 vs ボス: ヒット判定に必要なデータをコピー
            eff.boss_invincible = boss.invincible;
            eff.boss_r          = bp.radius;
            eff.boss_exp_reward = bp.exp_reward;
            eff.boss_x          = boss.x;
            eff.boss_y          = boss.y;
            eff.boss_hp_ref     = boss.hp;
        }
        // boss 借用をここで解放してから弾丸データにアクセス

        // 弾丸 vs ボス: boss 借用の外で処理
        if w.boss.is_some() && !eff.boss_invincible {
            let bullet_len = w.bullets.positions_x.len();
            for bi in 0..bullet_len {
                if !w.bullets.alive[bi] { continue; }
                let dmg = w.bullets.damage[bi];
                if dmg == 0 { continue; }
                let bx = w.bullets.positions_x[bi];
                let by = w.bullets.positions_y[bi];
                let hit_r2 = BULLET_RADIUS + eff.boss_r;
                let ddx2 = bx - eff.boss_x;
                let ddy2 = by - eff.boss_y;
                if ddx2 * ddx2 + ddy2 * ddy2 < hit_r2 * hit_r2 {
                    eff.bullet_hits.push((bi, dmg as f32, !w.bullets.piercing[bi]));
                }
            }
            // ダメージ適用
            let total_dmg: f32 = eff.bullet_hits.iter().map(|&(_, d, _)| d).sum();
            if total_dmg > 0.0 {
                if let Some(ref mut boss) = w.boss {
                    boss.hp -= total_dmg;
                    if boss.hp <= 0.0 {
                        eff.boss_killed = true;
                        eff.exp_reward  = eff.boss_exp_reward;
                        eff.kill_x      = boss.x;
                        eff.kill_y      = boss.y;
                    }
                }
            }
        }

        // ── フェーズ2: boss 借用を解放してから副作用を適用 ────────

        // プレイヤーダメージ
        if eff.hurt_player {
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                let dmg = eff.boss_damage * dt;
                w.player.hp = (w.player.hp - dmg).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
                w.frame_events.push(FrameEvent::PlayerDamaged { damage: dmg });
                w.particles.emit(eff.hurt_x, eff.hurt_y, 8, [1.0, 0.15, 0.15, 1.0]);
            }
        }

        // 弾丸ヒットパーティクル & 弾丸消去
        if !eff.bullet_hits.is_empty() {
            w.particles.emit(eff.boss_x, eff.boss_y, 4, [1.0, 0.8, 0.2, 1.0]);
            for &(bi, _, kill_bullet) in &eff.bullet_hits {
                if kill_bullet { w.bullets.kill(bi); }
            }
        }

        // 特殊行動の副作用
        if eff.spawn_slimes {
            let positions: Vec<(f32, f32)> = (0..8).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                (eff.special_x + angle.cos() * 120.0, eff.special_y + angle.sin() * 120.0)
            }).collect();
            w.enemies.spawn(&positions, 0); // Slime
            w.particles.emit(eff.special_x, eff.special_y, 16, [0.2, 1.0, 0.2, 1.0]);
        }
        if eff.spawn_rocks {
            for (dx_dir, dy_dir) in [(1.0_f32, 0.0_f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                w.bullets.spawn_ex(eff.special_x, eff.special_y, dx_dir * 200.0, dy_dir * 200.0, 50, 3.0, false, BULLET_KIND_ROCK, 0);
            }
            w.particles.emit(eff.special_x, eff.special_y, 10, [0.6, 0.6, 0.6, 1.0]);
        }
        if eff.bat_dash {
            w.particles.emit(eff.special_x, eff.special_y, 12, [0.8, 0.2, 1.0, 1.0]);
        }
        if eff.boss_killed {
            let boss_k = w.boss.as_ref().map(|b| b.kind_id).unwrap_or(0);
            w.kill_count += 1;
            w.score_popups.push((eff.kill_x, eff.kill_y - 20.0, eff.exp_reward * 2, 0.8));
            w.frame_events.push(FrameEvent::BossDefeated { boss_kind: boss_k });
            w.score += eff.exp_reward * 2;
            w.exp   += eff.exp_reward;
            if !w.level_up_pending {
                let required = exp_required_for_next(w.level);
                if w.exp >= required {
                    let new_lv = w.level + 1;
                    w.level_up_pending = true;
                    w.weapon_choices = compute_weapon_choices(w);
                    w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                }
            }
            w.particles.emit(eff.kill_x, eff.kill_y, 40, [1.0, 0.5, 0.0, 1.0]);
            for _ in 0..10 {
                let ox = (w.rng.next_f32() - 0.5) * 200.0;
                let oy = (w.rng.next_f32() - 0.5) * 200.0;
                w.items.spawn(eff.kill_x + ox, eff.kill_y + oy, ItemKind::Gem, eff.exp_reward / 10);
            }
            w.boss = None;
        }
    }

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
