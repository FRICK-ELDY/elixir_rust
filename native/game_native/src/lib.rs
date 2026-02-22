mod constants;
mod physics;

use constants::{PLAYER_SIZE, PLAYER_SPEED, SCREEN_HEIGHT, SCREEN_WIDTH};
use physics::spatial_hash::CollisionWorld;
use rustler::{Atom, NifResult, ResourceArc};
use std::sync::Mutex;

rustler::atoms! {
    ok,
    slime,
}

// ─── 定数 ─────────────────────────────────────────────────────
/// プレイヤーの当たり判定半径（px）
const PLAYER_RADIUS: f32 = PLAYER_SIZE / 2.0;
/// 敵の当たり判定半径（px）
const ENEMY_RADIUS: f32 = 20.0;
/// 敵がプレイヤーに触れたときのダメージ（HP/秒）
const ENEMY_DAMAGE_PER_SEC: f32 = 20.0;
/// 被弾後の無敵時間（秒）
const INVINCIBLE_DURATION: f32 = 0.5;
/// Spatial Hash のセルサイズ（px）
const CELL_SIZE: f32 = 80.0;
/// Magic Wand の発射間隔（秒）
const WEAPON_COOLDOWN: f32 = 1.0;
/// 弾丸の移動速度（px/秒）
const BULLET_SPEED: f32 = 400.0;
/// 弾丸のダメージ
const BULLET_DAMAGE: i32 = 10;
/// 弾丸の生存時間（秒）
const BULLET_LIFETIME: f32 = 3.0;
/// 弾丸の当たり判定半径（px）
const BULLET_RADIUS: f32 = 6.0;

// ─── プレイヤー ───────────────────────────────────────────────
pub struct PlayerState {
    pub x:                f32,
    pub y:                f32,
    pub input_dx:         f32,
    pub input_dy:         f32,
    pub hp:               f32,
    pub invincible_timer: f32,
}

// ─── 敵 SoA ──────────────────────────────────────────────────
pub struct EnemyWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub speeds:       Vec<f32>,
    pub hp:           Vec<f32>,
    pub alive:        Vec<bool>,
}

impl EnemyWorld {
    pub fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            speeds:       Vec::new(),
            hp:           Vec::new(),
            alive:        Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }

    /// 画面外のランダムな位置に `n` 体スポーン
    /// positions: spawn_enemies で事前生成した座標リスト
    pub fn spawn(&mut self, positions: &[(f32, f32)]) {
        for &(x, y) in positions {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(0.0);
            self.velocities_y.push(0.0);
            self.speeds.push(80.0);
            self.hp.push(30.0);
            self.alive.push(true);
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

// ─── 弾丸 SoA ─────────────────────────────────────────────────
pub struct BulletWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage:       Vec<i32>,
    pub lifetime:     Vec<f32>,
    pub alive:        Vec<bool>,
    pub count:        usize,
}

impl BulletWorld {
    pub fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            damage:       Vec::new(),
            lifetime:     Vec::new(),
            alive:        Vec::new(),
            count:        0,
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, damage: i32, lifetime: f32) {
        // 死んでいるスロットを再利用
        for i in 0..self.positions_x.len() {
            if !self.alive[i] {
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = vx;
                self.velocities_y[i] = vy;
                self.damage[i]       = damage;
                self.lifetime[i]     = lifetime;
                self.alive[i]        = true;
                self.count += 1;
                return;
            }
        }
        self.positions_x.push(x);
        self.positions_y.push(y);
        self.velocities_x.push(vx);
        self.velocities_y.push(vy);
        self.damage.push(damage);
        self.lifetime.push(lifetime);
        self.alive.push(true);
        self.count += 1;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }
}

// ─── 武器状態 ─────────────────────────────────────────────────
pub struct WeaponState {
    /// 次の発射まで残り時間（秒）
    pub cooldown_timer: f32,
}

impl WeaponState {
    pub fn new() -> Self {
        Self { cooldown_timer: 0.0 }
    }
}

/// 最近接の生存敵インデックスを返す
pub fn find_nearest_enemy(enemies: &EnemyWorld, px: f32, py: f32) -> Option<usize> {
    let mut min_dist = f32::MAX;
    let mut nearest  = None;
    for i in 0..enemies.len() {
        if !enemies.alive[i] {
            continue;
        }
        let dx   = enemies.positions_x[i] - px;
        let dy   = enemies.positions_y[i] - py;
        let dist = dx * dx + dy * dy;
        if dist < min_dist {
            min_dist = dist;
            nearest  = Some(i);
        }
    }
    nearest
}

/// Chase AI: 全敵をプレイヤーに向けて移動
pub fn update_chase_ai(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    for i in 0..enemies.len() {
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
    pub bullets:   BulletWorld,
    pub weapon:    WeaponState,
    pub rng:       SimpleRng,
    pub collision: CollisionWorld,
}

impl GameWorldInner {
    /// 衝突判定用の Spatial Hash を再構築する（clone 不要）
    fn rebuild_collision(&mut self) {
        self.collision.dynamic.clear();
        self.enemies.alive
            .iter()
            .enumerate()
            .filter(|&(_, &is_alive)| is_alive)
            .for_each(|(i, _)| {
                self.collision.dynamic.insert(
                    i,
                    self.enemies.positions_x[i],
                    self.enemies.positions_y[i],
                );
            });
    }
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
            x:                SCREEN_WIDTH  / 2.0 - PLAYER_SIZE / 2.0,
            y:                SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
            input_dx:         0.0,
            input_dy:         0.0,
            hp:               100.0,
            invincible_timer: 0.0,
        },
        enemies:   EnemyWorld::new(),
        bullets:   BulletWorld::new(),
        weapon:    WeaponState::new(),
        rng:       SimpleRng::new(12345),
        collision: CollisionWorld::new(CELL_SIZE),
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
    // rng の借用を先に終わらせてから enemies に渡す
    let positions: Vec<(f32, f32)> = (0..count)
        .map(|_| spawn_position_outside(&mut w.rng, SCREEN_WIDTH, SCREEN_HEIGHT))
        .collect();
    w.enemies.spawn(&positions);
    ok()
}

/// 物理ステップ: プレイヤー移動 + Chase AI + 衝突判定（Step 8/9/10）
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
    let px = w.player.x + PLAYER_RADIUS;
    let py = w.player.y + PLAYER_RADIUS;
    update_chase_ai(&mut w.enemies, px, py, dt);

    // ── Step 10: 衝突判定（Spatial Hash）────────────────────────
    // 1. 動的 Spatial Hash を再構築
    w.rebuild_collision();

    // 2. プレイヤー周辺の敵を取得して円-円判定
    let hit_radius = PLAYER_RADIUS + ENEMY_RADIUS;
    let candidates = w.collision.dynamic.query_nearby(px, py, hit_radius);

    // 無敵タイマーを更新
    if w.player.invincible_timer > 0.0 {
        w.player.invincible_timer = (w.player.invincible_timer - dt).max(0.0);
    }

    for idx in candidates {
        if !w.enemies.alive[idx] {
            continue;
        }
        let ex = w.enemies.positions_x[idx] + ENEMY_RADIUS;
        let ey = w.enemies.positions_y[idx] + ENEMY_RADIUS;
        let ddx = px - ex;
        let ddy = py - ey;
        let dist_sq = ddx * ddx + ddy * ddy;

        if dist_sq < hit_radius * hit_radius {
            // 敵→プレイヤーへのダメージ（無敵時間中は無効）
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                w.player.hp = (w.player.hp - ENEMY_DAMAGE_PER_SEC * dt).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
            }
        }
    }

    // ── Step 11: 武器・弾丸システム ──────────────────────────────
    // 1. 武器クールダウンを更新し、発射タイミングなら最近接敵に向けて発射
    w.weapon.cooldown_timer = (w.weapon.cooldown_timer - dt).max(0.0);
    if w.weapon.cooldown_timer <= 0.0 {
        if let Some(target_idx) = find_nearest_enemy(&w.enemies, px, py) {
            let tx  = w.enemies.positions_x[target_idx] + ENEMY_RADIUS;
            let ty  = w.enemies.positions_y[target_idx] + ENEMY_RADIUS;
            let bdx = tx - px;
            let bdy = ty - py;
            let blen = (bdx * bdx + bdy * bdy).sqrt().max(0.001);
            let vx  = (bdx / blen) * BULLET_SPEED;
            let vy  = (bdy / blen) * BULLET_SPEED;
            w.bullets.spawn(px, py, vx, vy, BULLET_DAMAGE, BULLET_LIFETIME);
            w.weapon.cooldown_timer = WEAPON_COOLDOWN;
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
        // 画面外に出た弾丸も消す
        let bx = w.bullets.positions_x[i];
        let by = w.bullets.positions_y[i];
        if bx < -100.0 || bx > SCREEN_WIDTH + 100.0 || by < -100.0 || by > SCREEN_HEIGHT + 100.0 {
            w.bullets.kill(i);
        }
    }

    // 3. 弾丸 vs 敵 衝突判定
    let hit_r = BULLET_RADIUS + ENEMY_RADIUS;
    for bi in 0..bullet_len {
        if !w.bullets.alive[bi] {
            continue;
        }
        let bx  = w.bullets.positions_x[bi];
        let by  = w.bullets.positions_y[bi];
        let dmg = w.bullets.damage[bi];

        let nearby = w.collision.dynamic.query_nearby(bx, by, hit_r);
        for ei in nearby {
            if !w.enemies.alive[ei] {
                continue;
            }
            let ex  = w.enemies.positions_x[ei] + ENEMY_RADIUS;
            let ey  = w.enemies.positions_y[ei] + ENEMY_RADIUS;
            let ddx = bx - ex;
            let ddy = by - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                w.enemies.hp[ei] -= dmg as f32;
                if w.enemies.hp[ei] <= 0.0 {
                    w.enemies.alive[ei] = false;
                }
                w.bullets.kill(bi);
                break;
            }
        }
    }

    w.frame_id
}

/// プレイヤー座標を返す（Step 8）
#[rustler::nif]
fn get_player_pos(world: ResourceArc<GameWorld>) -> (f64, f64) {
    let w = world.0.lock().unwrap();
    (w.player.x as f64, w.player.y as f64)
}

/// プレイヤー HP を返す（Step 10）
#[rustler::nif]
fn get_player_hp(world: ResourceArc<GameWorld>) -> f64 {
    let w = world.0.lock().unwrap();
    w.player.hp as f64
}

/// 描画データを返す: [{x, y, kind}] のリスト
/// kind: 0 = player, 1 = enemy, 2 = bullet
#[rustler::nif]
fn get_render_data(world: ResourceArc<GameWorld>) -> Vec<(f32, f32, u8)> {
    let w = world.0.lock().unwrap();
    let mut result = Vec::with_capacity(1 + w.enemies.len() + w.bullets.len());
    result.push((w.player.x, w.player.y, 0u8));
    for i in 0..w.enemies.len() {
        if w.enemies.alive[i] {
            result.push((w.enemies.positions_x[i], w.enemies.positions_y[i], 1u8));
        }
    }
    for i in 0..w.bullets.len() {
        if w.bullets.alive[i] {
            result.push((w.bullets.positions_x[i], w.bullets.positions_y[i], 2u8));
        }
    }
    result
}

/// 現在飛んでいる弾丸数を返す（Step 11）
#[rustler::nif]
fn get_bullet_count(world: ResourceArc<GameWorld>) -> usize {
    let w = world.0.lock().unwrap();
    w.bullets.count
}

// ─── ローダー ─────────────────────────────────────────────────

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);

// ─── 簡易 LCG 乱数生成器 ──────────────────────────────────────
pub struct SimpleRng(u64);

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
