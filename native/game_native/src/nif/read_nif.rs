//! Path: native/game_native/src/nif/read_nif.rs
//! Summary: 読み取り専用 NIF（get_*、debug_dump_world、is_player_dead）

use super::util::lock_poisoned_err;
use crate::world::GameWorld;
use game_core::entity_params::{BossParams, EnemyParams, WeaponParams};
use game_core::util::exp_required_for_next;
use rustler::{Atom, NifResult, ResourceArc};

use crate::{alive, none};

#[rustler::nif]
pub fn get_player_pos(world: ResourceArc<GameWorld>) -> NifResult<(f64, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok((w.player.x as f64, w.player.y as f64))
}

#[rustler::nif]
pub fn get_player_hp(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.player.hp as f64)
}

#[deprecated(
    since = "0.1.0",
    note = "毎フレーム呼び出すと NIF オーバーヘッドが発生。get_frame_metadata でメタデータのみ取得すること"
)]
#[rustler::nif]
pub fn get_render_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, u8)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let mut result = Vec::with_capacity(1 + w.enemies.len() + w.bullets.len() + 1);
    result.push((w.player.x, w.player.y, 0u8));
    if let Some(ref boss) = w.boss {
        let bp = BossParams::get(boss.kind_id);
        let boss_sprite_size = if boss.kind_id == 2 { 128.0 } else { 96.0 };
        result.push((
            boss.x - boss_sprite_size / 2.0,
            boss.y - boss_sprite_size / 2.0,
            bp.render_kind,
        ));
    }
    for i in 0..w.enemies.len() {
        if w.enemies.alive[i] {
            result.push((
                w.enemies.positions_x[i],
                w.enemies.positions_y[i],
                EnemyParams::get(w.enemies.kind_ids[i]).render_kind,
            ));
        }
    }
    for i in 0..w.bullets.len() {
        if w.bullets.alive[i] {
            result.push((w.bullets.positions_x[i], w.bullets.positions_y[i], w.bullets.render_kind[i]));
        }
    }
    Ok(result)
}

#[deprecated(
    since = "0.1.0",
    note = "毎フレーム呼び出すと NIF オーバーヘッドが発生。描画は Rust 内で完結させること"
)]
#[rustler::nif]
pub fn get_particle_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, f32, f32, f32, f32, f32)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let mut result = Vec::with_capacity(w.particles.count);
    for i in 0..w.particles.len() {
        if !w.particles.alive[i] { continue; }
        let alpha = (w.particles.lifetime[i] / w.particles.max_lifetime[i]).clamp(0.0, 1.0);
        let c = w.particles.color[i];
        result.push((
            w.particles.positions_x[i],
            w.particles.positions_y[i],
            c[0], c[1], c[2],
            alpha,
            w.particles.size[i],
        ));
    }
    Ok(result)
}

#[rustler::nif]
pub fn get_bullet_count(world: ResourceArc<GameWorld>) -> NifResult<usize> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.bullets.count)
}

#[rustler::nif]
pub fn get_frame_time_ms(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.last_frame_time_ms)
}

#[cfg(debug_assertions)]
#[rustler::nif]
pub fn debug_dump_world(world: ResourceArc<GameWorld>) -> NifResult<String> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let boss_str = match &w.boss {
        Some(b) => format!("boss hp={:.0}/{:.0}", b.hp, b.max_hp),
        None => "boss=none".to_string(),
    };
    Ok(format!(
        "enemies={} bullets={} player=({:.1},{:.1}) hp={:.0}/{:.0} {}",
        w.enemies.count, w.bullets.count, w.player.x, w.player.y,
        w.player.hp, w.player_max_hp, boss_str
    ))
}

#[cfg(not(debug_assertions))]
#[rustler::nif]
pub fn debug_dump_world(_world: ResourceArc<GameWorld>) -> NifResult<String> {
    Err(rustler::Error::Atom("debug_build_only"))
}

#[rustler::nif]
pub fn get_enemy_count(world: ResourceArc<GameWorld>) -> NifResult<usize> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.enemies.count)
}

#[rustler::nif]
pub fn get_hud_data(world: ResourceArc<GameWorld>) -> NifResult<(f64, f64, u32, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok((
        w.player.hp        as f64,
        w.player_max_hp    as f64,
        w.score,
        w.elapsed_seconds  as f64,
    ))
}

#[rustler::nif]
pub fn get_frame_metadata(world: ResourceArc<GameWorld>) -> NifResult<(
    (f64, f64, u32, f64),
    (usize, usize, f64),
    (u32, u32, bool, u32),
    (bool, f64, f64),
)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let exp_to_next = exp_required_for_next(w.level).saturating_sub(w.exp);
    let (boss_alive, boss_hp, boss_max_hp) = match &w.boss {
        Some(boss) => (true, boss.hp as f64, boss.max_hp as f64),
        None       => (false, 0.0, 0.0),
    };
    Ok((
        (w.player.hp as f64, w.player_max_hp as f64, w.score, w.elapsed_seconds as f64),
        (w.enemies.count, w.bullets.count, w.last_frame_time_ms),
        (w.exp, w.level, w.level_up_pending, exp_to_next),
        (boss_alive, boss_hp, boss_max_hp),
    ))
}

#[rustler::nif]
pub fn get_level_up_data(world: ResourceArc<GameWorld>) -> NifResult<(u32, u32, bool, u32)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let exp_to_next = exp_required_for_next(w.level).saturating_sub(w.exp);
    Ok((w.exp, w.level, w.level_up_pending, exp_to_next))
}

#[rustler::nif]
pub fn get_weapon_levels(world: ResourceArc<GameWorld>) -> NifResult<Vec<(String, u32)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.weapon_slots.iter()
        .map(|s| (WeaponParams::get(s.kind_id).name.to_string(), s.level))
        .collect())
}

#[deprecated(
    since = "0.1.0",
    note = "毎フレーム呼び出すと NIF オーバーヘッドが発生。描画は Rust 内で完結させること"
)]
#[rustler::nif]
pub fn get_item_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, u8)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let mut result = Vec::with_capacity(w.items.count);
    for i in 0..w.items.len() {
        if w.items.alive[i] {
            result.push((
                w.items.positions_x[i],
                w.items.positions_y[i],
                w.items.kinds[i].render_kind(),
            ));
        }
    }
    Ok(result)
}

#[rustler::nif]
pub fn get_magnet_timer(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.magnet_timer as f64)
}

#[rustler::nif]
pub fn get_boss_info(world: ResourceArc<GameWorld>) -> NifResult<(Atom, f64, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(match &w.boss {
        Some(boss) => (alive(), boss.hp as f64, boss.max_hp as f64),
        None       => (none(), 0.0, 0.0),
    })
}

#[rustler::nif]
pub fn is_player_dead(world: ResourceArc<GameWorld>) -> NifResult<bool> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.player.hp <= 0.0)
}
