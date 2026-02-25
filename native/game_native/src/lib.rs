//! Path: native/game_native/src/lib.rs
//! Summary: NIF エントリ・モジュール宣言・pub use・rustler::init のみ（スリム化済み）

pub use game_core::boss::BossKind;
pub use game_core::enemy::EnemyKind;

rustler::atoms! {
    ok,
    slime,
    bat,
    golem,
    // 武器種別アトム
    magic_wand,
    axe,
    cross,
    whip,
    fireball,
    lightning,
    // level_up 通知アトム
    level_up,
    no_change,
    // ボス種別アトム
    slime_king,
    bat_lord,
    stone_golem,
    // ゲーム状態アトム
    alive,
    dead,
    none,
    // イベントバス用アトム
    enemy_killed,
    player_damaged,
    level_up_event,
    item_pickup,
    boss_defeated,
    // Rust ゲームループ → Elixir 送信用
    frame_events,
}

mod asset;
mod audio;
mod game_logic;
mod nif;
mod renderer;
mod render_thread;
mod world;

pub use asset::{AssetId, AssetLoader};
pub use audio::AudioManager;
pub use game_logic::{
    find_nearest_enemy, find_nearest_enemy_excluding, find_nearest_enemy_spatial,
    find_nearest_enemy_spatial_excluding, update_chase_ai, update_chase_ai_simd,
};
pub use nif::{SaveSnapshot, WeaponSlotSave};
pub use renderer::{BossHudInfo, GamePhase, GameUiState, HudData, Renderer};
pub use world::{
    BossState, BulletWorld, EnemyWorld, FrameEvent, GameLoopControl, GameWorld, GameWorldInner,
    ParticleWorld, PlayerState,
    BULLET_KIND_FIREBALL, BULLET_KIND_LIGHTNING, BULLET_KIND_NORMAL, BULLET_KIND_ROCK,
    BULLET_KIND_WHIP,
};

rustler::init!("Elixir.App.NifBridge", load = nif::load);
