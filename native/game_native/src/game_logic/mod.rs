//! Path: native/game_native/src/game_logic/mod.rs
//! Summary: 物理ステップ・Chase AI・イベント drain

mod chase_ai;
mod events;
mod physics_step;

pub use chase_ai::{
    find_nearest_enemy, find_nearest_enemy_excluding, find_nearest_enemy_spatial,
    find_nearest_enemy_spatial_excluding, update_chase_ai, update_chase_ai_simd,
};
pub(crate) use events::drain_frame_events_inner;
pub(crate) use physics_step::{get_spawn_positions_around_player, physics_step_inner};
