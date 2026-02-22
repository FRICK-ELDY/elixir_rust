mod constants;

use constants::{PLAYER_SIZE, PLAYER_SPEED, SCREEN_HEIGHT, SCREEN_WIDTH};
use rustler::{Atom, NifResult, ResourceArc};
use std::sync::Mutex;

rustler::atoms! {
    ok,
    slime,
}

// ─── プレイヤー ───────────────────────────────────────────────
pub struct PlayerState {
    pub x:        f32,
    pub y:        f32,
    pub input_dx: f32,
    pub input_dy: f32,
}

// ─── 敵 SoA ──────────────────────────────────────────────────
pub struct EnemyWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub speeds:       Vec<f32>,
    pub alive:        Vec<bool>,
    pub count:        usize,
}

impl EnemyWorld {
    pub fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            speeds:       Vec::new(),
            alive:        Vec::new(),
            count:        0,
        }
    }

    /// 画面外のランダムな位置に `n` 体スポーン
    pub fn spawn(&mut self, n: usize, screen_w: f32, screen_h: f32, rng_seed: u64) {
        let mut rng = SimpleRng::new(rng_seed);
        for _ in 0..n {
            let (x, y) = spawn_position_outside(&mut rng, screen_w, screen_h);
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(0.0);
            self.velocities_y.push(0.0);
            self.speeds.push(80.0);
            self.alive.push(true);
            self.count += 1;
        }
    }
}

/// 画面外の四辺いずれかにランダムに配置
fn spawn_position_outside(rng: &mut SimpleRng, sw: f32, sh: f32) -> (f32, f32) {
    let margin = 80.0;
    let side = rng.next_u32() % 4;
    match side {
        0 => (rng.next_f32() * sw, -margin),
        1 => (rng.next_f32() * sw, sh + margin),
        2 => (-margin,             rng.next_f32() * sh),
        _ => (sw + margin,         rng.next_f32() * sh),
    }
}

/// Chase AI: 全敵をプレイヤーに向けて移動
pub fn update_chase_ai(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    for i in 0..enemies.count {
        if !enemies.alive[i] {
            continue;
        }
        let dx = player_x - enemies.positions_x[i];
        let dy = player_y - enemies.positions_y[i];
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        enemies.velocities_x[i] = (dx / dist) * enemies.speeds[i];
        enemies.velocities_y[i] = (dy / dist) * enemies.speeds[i];
        enemies.positions_x[i] += enemies.velocities_x[i] * dt;
        enemies.positions_y[i] += enemies.velocities_y[i] * dt;
    }
}

// ─── ゲームワールド ───────────────────────────────────────────
pub struct GameWorldInner {
    pub frame_id:  u32,
    pub player:    PlayerState,
    pub enemies:   EnemyWorld,
    pub rng_state: u64,
}

pub struct GameWorld(pub Mutex<GameWorldInner>);

// ─── NIF 関数 ─────────────────────────────────────────────────

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

#[rustler::nif]
fn create_world() -> ResourceArc<GameWorld> {
    ResourceArc::new(GameWorld(Mutex::new(GameWorldInner {
        frame_id:  0,
        player:    PlayerState {
            x:        SCREEN_WIDTH  / 2.0 - PLAYER_SIZE / 2.0,
            y:        SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
            input_dx: 0.0,
            input_dy: 0.0,
        },
        enemies:   EnemyWorld::new(),
        rng_state: 12345,
    })))
}

/// プレイヤーの入力方向を設定（Step 8）
#[rustler::nif]
fn set_player_input(world: ResourceArc<GameWorld>, dx: f64, dy: f64) -> Atom {
    let mut w = world.0.lock().unwrap();
    w.player.input_dx = dx as f32;
    w.player.input_dy = dy as f32;
    ok()
}

/// 敵をスポーン（Step 9）
#[rustler::nif]
fn spawn_enemies(world: ResourceArc<GameWorld>, _kind: Atom, count: usize) -> Atom {
    let mut w = world.0.lock().unwrap();
    let seed = w.rng_state;
    w.rng_state = w.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    w.enemies.spawn(count, SCREEN_WIDTH, SCREEN_HEIGHT, seed);
    ok()
}

/// 物理ステップ: プレイヤー移動 + Chase AI（Step 8/9）
#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> u32 {
    let mut w = world.0.lock().unwrap();
    w.frame_id += 1;

    let dt = delta_ms as f32 / 1000.0;
    let dx = w.player.input_dx;
    let dy = w.player.input_dy;

    // 斜め移動を正規化して速度を一定に保つ
    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.001 {
        w.player.x += (dx / len) * PLAYER_SPEED * dt;
        w.player.y += (dy / len) * PLAYER_SPEED * dt;
    }

    w.player.x = w.player.x.clamp(0.0, SCREEN_WIDTH  - PLAYER_SIZE);
    w.player.y = w.player.y.clamp(0.0, SCREEN_HEIGHT - PLAYER_SIZE);

    // Chase AI
    let px = w.player.x;
    let py = w.player.y;
    update_chase_ai(&mut w.enemies, px, py, dt);

    w.frame_id
}

/// プレイヤー座標を返す（Step 8）
#[rustler::nif]
fn get_player_pos(world: ResourceArc<GameWorld>) -> (f64, f64) {
    let w = world.0.lock().unwrap();
    (w.player.x as f64, w.player.y as f64)
}

/// 描画データを返す: [{x, y, kind}] のリスト
/// kind: 0 = player, 1 = enemy
#[rustler::nif]
fn get_render_data(world: ResourceArc<GameWorld>) -> Vec<(f32, f32, u8)> {
    let w = world.0.lock().unwrap();
    let mut result = Vec::with_capacity(1 + w.enemies.count);
    result.push((w.player.x, w.player.y, 0u8));
    for i in 0..w.enemies.count {
        if w.enemies.alive[i] {
            result.push((w.enemies.positions_x[i], w.enemies.positions_y[i], 1u8));
        }
    }
    result
}

// ─── ローダー ─────────────────────────────────────────────────

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);

// ─── 簡易 LCG 乱数生成器 ──────────────────────────────────────
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}
