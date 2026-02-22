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
pub const MAX_ENEMIES: usize = 10_000;

// Wave-based spawn schedule: (start_secs, interval_secs, count_per_tick)
// Used by game_window binary; Elixir SpawnSystem handles this for the NIF lib.
#[allow(dead_code)]
pub const WAVES: &[(f32, f32, usize)] = &[
    (  0.0, 0.8,   20),
    ( 10.0, 0.6,   50),
    ( 30.0, 0.4,  100),
    ( 60.0, 0.3,  200),
    (120.0, 0.2,  300),
];
