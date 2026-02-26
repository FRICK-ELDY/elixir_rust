//! Path: native/game_native/src/render_snapshot.rs
//! Summary: GameWorld から描画用スナップショットを構築（1.7.5）
//!
//! 描画スレッド内で world.read() を保持する時間を最小化するため、
//! 必要なデータを RenderSnapshot にコピーしてからロックを解放する。

use crate::world::GameWorldInner;
use game_render::{BossHudInfo, GamePhase, HudData, RenderFrame};
use game_core::constants::{INVINCIBLE_DURATION, PLAYER_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH};
use game_core::entity_params::{BossParams, EnemyParams, WeaponParams};
use game_core::util::exp_required_for_next;

/// GameWorldInner から RenderSnapshot を構築する。
/// get_render_data / get_particle_data / get_item_data / get_frame_metadata 相当のロジックを集約。
pub fn build_render_frame(w: &GameWorldInner) -> RenderFrame {
    // 1. スプライト（player, boss, enemies, bullets）
    let anim_frame = ((w.frame_id / 4) % 4) as u8;
    let mut render_data = Vec::with_capacity(
        1 + w.boss.is_some() as usize + w.enemies.count + w.bullets.count,
    );

    render_data.push((w.player.x, w.player.y, 0, anim_frame));

    if let Some(ref boss) = w.boss {
        let bp = BossParams::get(boss.kind_id);
        let boss_sprite_size = bp.radius * 2.0;
        render_data.push((
            boss.x - boss_sprite_size / 2.0,
            boss.y - boss_sprite_size / 2.0,
            bp.render_kind,
            0,
        ));
    }

    for i in 0..w.enemies.len() {
        if w.enemies.alive[i] {
            let base_kind = EnemyParams::get(w.enemies.kind_ids[i]).render_kind;
            // game_native の EnemyWorld には is_elite がないため、通常描画のみ
            render_data.push((
                w.enemies.positions_x[i],
                w.enemies.positions_y[i],
                base_kind,
                anim_frame,
            ));
        }
    }

    for i in 0..w.bullets.len() {
        if w.bullets.alive[i] {
            render_data.push((
                w.bullets.positions_x[i],
                w.bullets.positions_y[i],
                w.bullets.render_kind[i],
                0,
            ));
        }
    }

    // 2. パーティクル
    let mut particle_data = Vec::with_capacity(w.particles.count);
    for i in 0..w.particles.len() {
        if !w.particles.alive[i] {
            continue;
        }
        let alpha = (w.particles.lifetime[i] / w.particles.max_lifetime[i]).clamp(0.0, 1.0);
        let c = w.particles.color[i];
        particle_data.push((
            w.particles.positions_x[i],
            w.particles.positions_y[i],
            c[0], c[1], c[2],
            alpha,
            w.particles.size[i],
        ));
    }

    // 3. アイテム
    let mut item_data = Vec::with_capacity(w.items.count);
    for i in 0..w.items.len() {
        if w.items.alive[i] {
            item_data.push((
                w.items.positions_x[i],
                w.items.positions_y[i],
                w.items.kinds[i].render_kind(),
            ));
        }
    }

    // 4. 障害物（collision.obstacles から）
    let obstacle_data: Vec<(f32, f32, f32, u8)> = w.collision.obstacles
        .iter()
        .map(|o| (o.x, o.y, o.radius, o.kind))
        .collect();

    // 5. カメラオフセット（プレイヤー中心を画面中心に）
    let cam_x = w.player.x + PLAYER_SIZE / 2.0 - SCREEN_WIDTH / 2.0;
    let cam_y = w.player.y + PLAYER_SIZE / 2.0 - SCREEN_HEIGHT / 2.0;
    let camera_offset = (cam_x, cam_y);

    // 6. HUD メタデータ（get_frame_metadata 相当）
    let exp_to_next = exp_required_for_next(w.level).saturating_sub(w.exp);
    let boss_info = w.boss.as_ref().map(|b| BossHudInfo {
        name:   BossParams::get(b.kind_id).name.to_string(),
        hp:     b.hp,
        max_hp: b.max_hp,
    });

    let weapon_levels: Vec<(String, u32)> = w.weapon_slots
        .iter()
        .map(|s| (WeaponParams::get(s.kind_id).name.to_string(), s.level))
        .collect();

    let screen_flash_alpha = if w.player.invincible_timer > 0.0 && INVINCIBLE_DURATION > 0.0 {
        // 被弾直後に強く、無敵時間の減衰にあわせてフラッシュも弱くする（最大 0.5）
        ((w.player.invincible_timer / INVINCIBLE_DURATION).clamp(0.0, 1.0)) * 0.5
    } else {
        0.0
    };

    let hud = HudData {
        hp:               w.player.hp,
        max_hp:           w.player_max_hp,
        score:            w.score,
        elapsed_seconds:  w.elapsed_seconds,
        level:            w.level,
        exp:              w.exp,
        exp_to_next,
        enemy_count:      w.enemies.count,
        bullet_count:     w.bullets.count,
        fps:              0.0,
        level_up_pending: w.level_up_pending,
        weapon_choices:   w.weapon_choices.clone(),
        weapon_levels,
        magnet_timer:     w.magnet_timer,
        item_count:       w.items.count,
        camera_x:         cam_x,
        camera_y:         cam_y,
        boss_info,
        phase:            GamePhase::Playing,
        screen_flash_alpha,
        score_popups:     w.score_popups.clone(),
        kill_count:       w.kill_count,
    };

    RenderFrame {
        render_data,
        particle_data,
        item_data,
        obstacle_data,
        camera_offset,
        hud,
    }
}
