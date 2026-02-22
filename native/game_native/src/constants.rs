// Background clear color (dark purple)
#[allow(dead_code)]
pub const BG_R: f64 = 0.05;
#[allow(dead_code)]
pub const BG_G: f64 = 0.02;
#[allow(dead_code)]
pub const BG_B: f64 = 0.10;

// Window resolution
pub const SCREEN_WIDTH:  f32 = 1280.0;
pub const SCREEN_HEIGHT: f32 = 720.0;

// Map size (Step 20: camera scroll)
// Used by game_window binary; NIF lib uses fixed screen coordinates.
#[allow(dead_code)]
pub const MAP_WIDTH:  f32 = 4096.0;
#[allow(dead_code)]
pub const MAP_HEIGHT: f32 = 4096.0;

// Camera lerp speed (Step 20)
#[allow(dead_code)]
pub const CAMERA_LERP_SPEED: f32 = 5.0;

// Sprite / player size
pub const SPRITE_SIZE:  f32 = 64.0;
pub const PLAYER_SIZE:  f32 = SPRITE_SIZE;

// Movement
pub const PLAYER_SPEED: f32 = 200.0;

// Frame budget (used by lib.rs NIF; not all binaries reference it)
#[allow(dead_code)]
pub const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;

// Collision radii
pub const PLAYER_RADIUS: f32 = PLAYER_SIZE / 2.0;
pub const ENEMY_RADIUS:  f32 = 20.0;
pub const BULLET_RADIUS: f32 = 6.0;

// Enemy separation: 敵同士が重ならないための押し出し半径・強さ
pub const ENEMY_SEPARATION_RADIUS: f32 = ENEMY_RADIUS * 2.0;
pub const ENEMY_SEPARATION_FORCE:  f32 = 120.0;

// Combat
#[allow(dead_code)]
pub const ENEMY_DAMAGE_PER_SEC: f32 = 20.0;
pub const INVINCIBLE_DURATION:  f32 = 0.5;
pub const WEAPON_COOLDOWN:      f32 = 1.0;
pub const BULLET_SPEED:         f32 = 400.0;
pub const BULLET_DAMAGE:        i32 = 10;
pub const BULLET_LIFETIME:      f32 = 3.0;

// Spatial hash cell size
pub const CELL_SIZE: f32 = 80.0;

// Enemy cap (used by game_window binary; not referenced by the NIF lib)
#[allow(dead_code)]
pub const MAX_ENEMIES: usize = 300;

// Wave-based spawn schedule: (start_secs, interval_secs, count_per_tick)
// Used by game_window binary; Elixir SpawnSystem handles this for the NIF lib.
#[allow(dead_code)]
pub const WAVES: &[(f32, f32, usize)] = &[
    (  0.0, 3.0,   3),   //   0〜30s: 3体 / 3秒（チュートリアル）
    ( 30.0, 2.0,   5),   //  30〜60s: 5体 / 2秒（ウォームアップ）
    ( 60.0, 1.5,   8),   //  60〜120s: 8体 / 1.5秒（本番）
    (120.0, 1.0,  12),   // 120〜180s: 12体 / 1秒（激化）
    (180.0, 0.8,  15),   // 180s〜:   15体 / 0.8秒（最終盤）
];
